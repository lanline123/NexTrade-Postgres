//! 商户/店铺域服务：店铺信息、资金、提现、经营类目。
//! 对应原 Java `ShopServiceImpl` / `AccountServiceImpl` /
//! `ShopWithdrawServiceImpl` / `ShopProductCategoryServiceImpl` 等。
//!
//! | 方法 | 原 Java |
//! |------|---------|
//! | `list` / `detail` / `update` | `ShopServiceImpl.list` / `detailWithScore` / `update` |
//! | `account_logs` | `AccountServiceImpl.list` |
//! | `withdraw_list` / `withdraw_detail` / `withdraw_status_config` / `withdraw_audit` | `ShopWithdrawServiceImpl.list` / `detail` / `getStatusList` / `audit` |
//! | `product_categories` / `product_category_detail` / `category_tree` | `ShopProductCategoryServiceImpl.list` / `detail` / `tree` |
//! | `client_detail` | `ShopServiceImpl.clientDetail` |

use std::collections::HashMap;
use std::sync::Arc;

use rust_decimal::Decimal;
use nextrade_core::entities::{Merchant, Shop, ShopAccountLog, ShopWithdraw};
use nextrade_core::models::merchant::shop::{
    shop_status_text, withdraw_status_text, ClientShopDetailVO, ShopAccountLogListQuery,
    ShopAccountLogVO, ShopCategoryTreeNode, ShopListQuery, ShopMerchantAdminVO, ShopMerchantVO,
    ShopProductCategoryListQuery, ShopProductCategoryVO, ShopUpdateDTO, ShopVO,
    ShopWithdrawAuditDTO, ShopWithdrawListQuery, ShopWithdrawVO,
};
use nextrade_core::types::{PageQuery, PageResult};
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::Repositories;

pub struct ShopService {
    repos: Arc<Repositories>,
}

impl ShopService {
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    /// 店铺分页（`ShopController.list` → `ShopServiceImpl#list`）。
    ///
    /// `shopType` 缺省按 1 店铺过滤（原 `ShopListDTO.shopType` 默认 1）；
    /// 关键字/店铺名模糊匹配 `shop_title`，排除已删除店铺。
    pub async fn list(&self, q: &ShopListQuery) -> Result<PageResult<ShopVO>> {
        let page = PageQuery {
            page: q.page.unwrap_or(1),
            page_size: q.page_size.unwrap_or(20),
        };
        let keyword = first_of(&[q.keyword.as_deref(), q.shop_title.as_deref()]);
        let shop_type = Some(q.shop_type.unwrap_or(1));
        let status = q.status.filter(|s| *s > 0);
        let add_start = day_range(q.add_start_time.as_deref(), false)?;
        let add_end = day_range(q.add_end_time.as_deref(), true)?;

        let (total, rows) = self
            .repos.merchant.shop
            .paginate(
                keyword,
                shop_type,
                status,
                add_start,
                add_end,
                q.store_parent_id,
                q.shop_region_id.as_deref().filter(|s| !s.trim().is_empty()),
                q.sort_field.as_deref(),
                q.sort_order.as_deref(),
                &page,
            )
            .await?;

        // 原 Java 循环内逐行查商户/管理员，这里按当前页批量预取
        let (merchants, admins) = self.load_shop_refs(&rows).await?;
        let list = rows
            .iter()
            .map(|shop| {
                let mut vo = ShopVO::from(shop);
                let mut merchant = merchants.get(&shop.merchant_id.unwrap_or(0)).cloned();
                if let (Some(m), Some(admin)) = (merchant.as_mut(), admins.get(&shop.shop_id)) {
                    m.admin = Some(ShopMerchantAdminVO {
                        admin_id: admin.0,
                        username: admin.1.clone(),
                    });
                }
                vo.merchant = merchant;
                vo
            })
            .collect();
        Ok(PageResult::new(total, list, &page))
    }

    /// 店铺详情（`ShopController.detail` → `detailWithScore`）。
    ///
    /// 评星按 `ShopMapper#calculateShopStar`：评价数（`shop_star_num`）
    /// `< 100`/空/0 时固定 4.8，否则取 `shop_star`。
    pub async fn detail(&self, shop_id: i32) -> Result<ShopVO> {
        let shop = self
            .repos.merchant.shop
            .find_by_id(shop_id)
            .await?
            .ok_or_else(|| Error::NotFound("店铺不存在".into()))?;
        let mut vo = ShopVO::from(&shop);
        vo.shop_star = calc_shop_star(&shop);
        vo.product_score = Some(
            self.repos.merchant.shop
                .avg_comment_rank(shop_id)
                .await?
                .unwrap_or(Decimal::ZERO),
        );

        let (merchants, admins) = self.load_shop_refs(std::slice::from_ref(&shop)).await?;
        let mut merchant = merchants.get(&shop.merchant_id.unwrap_or(0)).cloned();
        if let (Some(m), Some(admin)) = (merchant.as_mut(), admins.get(&shop.shop_id)) {
            m.admin = Some(ShopMerchantAdminVO {
                admin_id: admin.0,
                username: admin.1.clone(),
            });
        }
        vo.merchant = merchant;
        Ok(vo)
    }

    /// 更新店铺（`ShopController.update` → `ShopServiceImpl#update`）。
    ///
    /// TODO(未迁移)：原 Java 还会重写门店标签 `store_tip` 并执行
    /// `checkStoreParam` 门店参数校验。
    pub async fn update(&self, dto: &ShopUpdateDTO) -> Result<()> {
        if dto.shop_id <= 0 {
            return Err(Error::Validation("店铺ID不能为空".into()));
        }
        let exists = self.repos.merchant.shop.find_by_id(dto.shop_id).await?;
        if exists.is_none() {
            return Err(Error::NotFound("店铺不存在".into()));
        }
        if !self.repos.merchant.shop.update_shop(dto).await? {
            return Err(Error::NotFound("店铺不存在".into()));
        }
        Ok(())
    }

    /// 资金流水分页（`AccountController.logList` → `AccountServiceImpl#list`）。
    pub async fn account_logs(
        &self,
        q: &ShopAccountLogListQuery,
    ) -> Result<PageResult<ShopAccountLogVO>> {
        let page = PageQuery {
            page: q.page.unwrap_or(1),
            page_size: q.page_size.unwrap_or(20),
        };

        let balance_type = match q.balance_type.as_deref().map(str::trim) {
            None | Some("") | Some("all") => None,
            Some("online") => Some(2),
            Some("offline") => Some(1),
            Some(_) => return Err(Error::Validation("资金类型仅支持 all、online、offline".into())),
        };
        let pay_channel = normalize_channel(q.pay_channel.as_deref())?;
        if pay_channel.is_some() && balance_type == Some(1) {
            return Err(Error::Validation("线下资金不支持线上渠道筛选".into()));
        }
        if let Some(t) = q.shop_type {
            if t != 1 && t != 2 {
                return Err(Error::Validation("店铺类型仅支持 1 店铺、2 门店".into()));
            }
        }
        let start = day_range(q.start_time.as_deref(), false)?;
        let end = day_range(q.end_time.as_deref(), true)?;
        let keyword = q.keyword.as_deref().map(str::trim).filter(|s| !s.is_empty());

        let (total, rows) = self
            .repos.merchant.shop
            .paginate_account_logs(
                balance_type,
                pay_channel.as_deref(),
                q.shop_id,
                q.shop_type,
                keyword,
                start,
                end,
                None,
                None,
                &page,
            )
            .await?;

        // 批量回填店铺标题（原 Java 逐行 `getById` 的 N+1 查询）
        let ids: Vec<i32> = {
            let mut ids: Vec<i32> = rows.iter().map(|l| l.shop_id).collect();
            ids.sort_unstable();
            ids.dedup();
            ids
        };
        let titles: HashMap<i32, String> = self
            .repos.merchant.shop
            .find_shop_titles(&ids)
            .await?
            .into_iter()
            .collect();

        let list = rows
            .iter()
            .map(|log| account_log_vo(log, titles.get(&log.shop_id).cloned()))
            .collect();
        Ok(PageResult::new(total, list, &page))
    }

    /// 提现记录分页（`ShopWithdrawController.list`）。
    ///
    /// TODO(数据隔离)：原 Java 按登录态 header 的 `shopId` 强制过滤，
    /// Rust 侧 admin 鉴权层未透出 shopId，故仅按 query 的 `shopId` 过滤。
    pub async fn withdraw_list(
        &self,
        q: &ShopWithdrawListQuery,
    ) -> Result<PageResult<ShopWithdrawVO>> {
        let page = PageQuery {
            page: q.page.unwrap_or(1),
            page_size: q.page_size.unwrap_or(20),
        };
        if let Some(t) = q.withdraw_type {
            if t != 1 && t != 2 {
                return Err(Error::Validation("提现类型只能为1或2".into()));
            }
        }
        let pay_channel = normalize_channel(q.pay_channel.as_deref())?;
        if pay_channel.is_some() && q.withdraw_type == Some(1) {
            return Err(Error::Validation("线下提现不支持线上渠道筛选".into()));
        }
        let start = day_range(q.add_time_start.as_deref(), false)?;
        let end = day_range(q.add_time_end.as_deref(), true)?;

        let (total, rows) = self
            .repos.merchant.shop
            .paginate_withdraws(
                q.status,
                q.withdraw_type,
                pay_channel.as_deref(),
                q.shop_id,
                start,
                end,
                q.sort_field.as_deref(),
                q.sort_order.as_deref(),
                &page,
            )
            .await?;
        let list = rows.iter().map(withdraw_vo).collect();
        Ok(PageResult::new(total, list, &page))
    }

    /// 提现详情（`ShopWithdrawController.detail`）。
    pub async fn withdraw_detail(&self, id: i32) -> Result<ShopWithdrawVO> {
        let w = self
            .repos.merchant.shop
            .find_withdraw(id)
            .await?
            .ok_or_else(|| Error::NotFound("提现记录不存在".into()))?;
        Ok(withdraw_vo(&w))
    }

    /// 提现状态配置（`ShopWithdrawController.config` → `getStatusList`）。
    ///
    /// 返回 `{"statusList": {"0": "待审核", ...}}`，隐藏 code=1 正在处理。
    pub fn withdraw_status_config(&self) -> serde_json::Value {
        let list = [
            (0, "待审核"),
            (2, "审核不通过"),
            (3, "完成"),
            (4, "待打款"),
        ];
        serde_json::json!({
            "statusList": serde_json::Map::from_iter(
                list.iter().map(|(code, name)| (code.to_string(), serde_json::Value::from(*name))),
            ),
        })
    }

    /// 审核提现（`ShopWithdrawController.audit` → `ShopWithdrawServiceImpl#audit`）。
    ///
    /// `admin_type` 必须为 `admin`（平台管理员），对应原 Java 校验
    /// `AdminRole.adminType == AdminTypeEnum.ADMIN`。
    pub async fn withdraw_audit(
        &self,
        dto: &ShopWithdrawAuditDTO,
        admin_type: Option<&str>,
    ) -> Result<()> {
        if admin_type != Some("admin") {
            return Err(Error::Forbidden("没有权限".into()));
        }
        let w = self
            .repos.merchant.shop
            .find_withdraw(dto.id)
            .await?
            .ok_or_else(|| Error::NotFound("提现记录不存在".into()))?;

        if w.withdraw_type == 2 {
            // 线上提现走渠道 API 审核（原 Java `auditOnlineWithdraw`），
            // 渠道出金能力未接入。
            return Err(Error::biz(400, "线上提现需通过渠道API审核，暂未接入"));
        }
        if w.status != 0 {
            return Err(Error::biz(400, "不是待审核状态"));
        }

        if dto.status == 2 {
            // 审核不通过：返还冻结金额并写流水（事务）
            let hit = self
                .repos.merchant.shop
                .reject_withdraw(&w, dto.audit_remark.as_deref())
                .await?;
            if !hit {
                return Err(Error::Conflict("不是待审核状态，请刷新后重试".into()));
            }
        } else {
            let hit = self
                .repos.merchant.shop
                .approve_withdraw(dto.id, dto.status, dto.audit_remark.as_deref())
                .await?;
            if !hit {
                return Err(Error::Conflict("不是待审核状态，请刷新后重试".into()));
            }
        }
        // TODO(财务域)：原 Java 审核后还会写 `statement` / `statement_output`
        // 对账单流水，属 finance 域，未在此接入。
        Ok(())
    }

    /// 店铺经营类目分页（`ShopProductCategoryController.list`）。
    ///
    /// 原 Java 从登录 header 取 `shopId` 覆盖入参，Rust 侧改为 query 传入。
    pub async fn product_categories(
        &self,
        q: &ShopProductCategoryListQuery,
    ) -> Result<PageResult<ShopProductCategoryVO>> {
        let page = PageQuery {
            page: q.page.unwrap_or(1),
            page_size: q.page_size.unwrap_or(20),
        };
        let keyword = q.keyword.as_deref().map(str::trim).filter(|s| !s.is_empty());
        let (total, rows) = self
            .repos.merchant.shop
            .paginate_product_categories(
                q.parent_id,
                q.shop_id,
                keyword,
                None,
                None,
                &page,
            )
            .await?;

        let ids: Vec<i32> = rows.iter().map(|c| c.category_id).collect();
        let children = self.repos.merchant.shop.count_children_map(&ids).await?;
        let list = rows
            .iter()
            .map(|c| {
                let mut vo = ShopProductCategoryVO::from(c);
                vo.has_children = if children.get(&c.category_id).copied().unwrap_or(0) > 0 {
                    1
                } else {
                    0
                };
                vo
            })
            .collect();
        Ok(PageResult::new(total, list, &page))
    }

    /// 经营类目详情（`ShopProductCategoryController.detail`）。
    pub async fn product_category_detail(&self, id: i32) -> Result<ShopProductCategoryVO> {
        let c = self
            .repos.merchant.shop
            .find_product_category(id)
            .await?
            .ok_or_else(|| Error::NotFound("该分类不存在".into()))?;
        Ok(ShopProductCategoryVO::from(&c))
    }

    /// 经营类目树（`ShopClientCategoryController.tree` → `tree(shopId)`，
    /// 亦对应管理端 `getAllCategory`）。
    ///
    /// 仅取 `is_show = 1`，按 `parent_id, sort_order, category_id` 升序，
    /// 以 `parent_id = 0` 为根；节点键对齐 hutool `Tree`（`isStore=1`）。
    pub async fn category_tree(&self, shop_id: i32) -> Result<Vec<ShopCategoryTreeNode>> {
        if shop_id <= 0 {
            return Err(Error::Validation("请选择店铺".into()));
        }
        let cats = self.repos.merchant.shop.find_categories_for_tree(shop_id).await?;
        if cats.is_empty() {
            return Ok(Vec::new());
        }
        Ok(build_category_tree(&cats, 0))
    }

    /// 前台店铺详情（`ShopClientController.detail` → `clientDetail`）。
    ///
    /// TODO(未接入)：装修位商品 `listingProduct`/`listingCount`、
    /// 营业执照图（`merchantService.getApprovedLicensePic`）、
    /// 平台自营（`shopId=0`）的配置名/logo 回填。
    pub async fn client_detail(&self, shop_id: i32) -> Result<ClientShopDetailVO> {
        if shop_id == 0 {
            // 平台自营店（shopId=0）不对应 shop 表记录
            return Ok(ClientShopDetailVO {
                shop_id: 0,
                ..Default::default()
            });
        }
        let shop = self
            .repos.merchant.shop
            .find_by_id(shop_id)
            .await?
            .ok_or_else(|| Error::NotFound("店铺不存在".into()))?;

        let mut vo = ClientShopDetailVO {
            status_text: shop_status_text(shop.status, shop.shop_type),
            shop_id: shop.shop_id,
            shop_title: shop.shop_title.clone().unwrap_or_default(),
            add_time: i64::from(shop.add_time.unwrap_or(0)) * 1000,
            shop_logo: shop.shop_logo.clone(),
            click_count: shop.click_count,
            status: shop.status,
            shop_type: shop.shop_type,
            merchant_id: shop.merchant_id,
            shop_money: shop.shop_money,
            frozen_money: shop.frozen_money,
            contact_mobile: shop.contact_mobile.clone(),
            description: shop.description.clone(),
            kefu_phone: shop.kefu_phone.clone(),
            kefu_weixin: shop.kefu_weixin.clone(),
            kefu_link: shop.kefu_link.clone(),
            is_contact_kefu: shop.is_contact_kefu,
            kefu_inlet: shop.kefu_inlet.clone(),
            shop_sales: shop.shop_sales,
            shop_star_num: shop.shop_star_num,
            shop_star: calc_shop_star(&shop),
            shop_region_id: shop.shop_region_id.clone(),
            shop_detailed_address: shop.shop_detailed_address.clone(),
            collect_count: self.repos.merchant.shop.count_collect_users(shop_id).await?,
            product_count: self.repos.merchant.shop.count_products(shop_id).await?,
            new_product_count: self.repos.merchant.shop.count_new_products(shop_id).await?,
            // 公开接口无登录态，未收藏
            collect_shop: false,
            product_score: self
                .repos.merchant.shop
                .avg_comment_rank(shop_id)
                .await?
                .unwrap_or(Decimal::ZERO),
            merchant: None,
            business_license_pic: String::new(),
        };

        if let Some(mid) = shop.merchant_id.filter(|id| *id > 0) {
            let merchants = self.repos.merchant.shop.find_merchants(&[mid]).await?;
            vo.merchant = merchants.first().map(|m| merchant_vo(m, true));
        }
        Ok(vo)
    }

    // ===== 私有辅助 =====

    /// 批量预取商户 + 店铺管理员（`list` / `detail` 共用）。
    async fn load_shop_refs(
        &self,
        shops: &[Shop],
    ) -> Result<(HashMap<i32, ShopMerchantVO>, HashMap<i32, (i32, String)>)> {
        let mut merchant_ids: Vec<i32> = shops
            .iter()
            .filter_map(|s| s.merchant_id)
            .filter(|id| *id > 0)
            .collect();
        merchant_ids.sort_unstable();
        merchant_ids.dedup();

        let mut shop_ids: Vec<i32> = shops.iter().map(|s| s.shop_id).collect();
        shop_ids.sort_unstable();
        shop_ids.dedup();

        let merchants = self.repos.merchant.shop.find_merchants(&merchant_ids).await?;
        let admins = self.repos.merchant.shop.find_shop_admins(&shop_ids).await?;
        Ok((
            merchants
                .iter()
                .map(|m| (m.merchant_id, merchant_vo(m, false)))
                .collect(),
            admins
                .into_iter()
                .map(|(shop_id, admin_id, username)| (shop_id, (admin_id, username)))
                .collect(),
        ))
    }
}

/// 资金流水行 → VO（`AccountServiceImpl#list` 的转换逻辑）。
fn account_log_vo(log: &ShopAccountLog, shop_title: Option<String>) -> ShopAccountLogVO {
    let change_amount = resolve_change_amount(log);
    let balance_type = log.balance_type;
    let (frozen, new_frozen) = if balance_type == 2 {
        // 线上流水不承载冻结余额（原 Java 置 0）
        (Decimal::ZERO, Decimal::ZERO)
    } else {
        (log.frozen_money, log.new_frozen_money)
    };

    // 原 Java 按 type 走 i18n key（默认中文文案）
    let change_reasons = if balance_type == 2 {
        log.remarks.clone()
    } else {
        match log.r#type.trim() {
            // 提现成功（余额不变）/ 提现驳回（返还）
            "1" => Some(
                if change_amount.is_some_and(|c| c.is_zero()) {
                    "提现成功"
                } else {
                    "提现驳回"
                }
                .to_string(),
            ),
            "2" => Some("提现申请".to_string()),
            "4" => Some("订单收支".to_string()),
            _ => None,
        }
    };

    ShopAccountLogVO {
        shop_account_log_id: log.shop_account_log_id,
        add_time: i64::from(log.add_time) * 1000,
        // 旧前端语义：shopMoney 与 changeAmount 一致
        shop_money: change_amount.unwrap_or(Decimal::ZERO),
        frozen_money: frozen,
        r#type: log.r#type.clone(),
        change_reasons,
        remarks: log.remarks.clone(),
        new_shop_money: log.new_shop_money,
        new_frozen_money: new_frozen,
        shop_id: log.shop_id,
        shop_title,
        change_amount,
        balance_type,
        pay_channel: log.pay_channel.clone(),
    }
}

/// 变动金额（`AccountServiceImpl#resolveChangeAmount`）：
/// 1. 显式 `change_amount` 优先；
/// 2. 新日志（`is_old = 1`）或提现日志（`type = 1`）按前后余额相减；
/// 3. 其余旧日志即 `shop_money` 本身。
fn resolve_change_amount(log: &ShopAccountLog) -> Option<Decimal> {
    if let Some(a) = log.change_amount {
        return Some(a);
    }
    if log.is_new == 1 || log.r#type.trim() == "1" {
        return log.new_shop_money.map(|n| n - log.shop_money);
    }
    Some(log.shop_money)
}

/// 提现记录 → VO。
///
/// TODO(手续费)：原 Java `fillRecordRate` 按 `record_rate` 表费率重算
/// `handlingFee`/`handlingRate`，此处直接取库列。
fn withdraw_vo(w: &ShopWithdraw) -> ShopWithdrawVO {
    ShopWithdrawVO {
        shop_withdraw_log_id: w.shop_withdraw_log_id,
        shop_id: w.shop_id,
        amount: w.amount,
        handling_fee: w.handling_fee,
        handling_rate: w.handling_rate,
        status: w.status,
        status_text: withdraw_status_text(w.status),
        add_time: i64::from(w.add_time) * 1000,
        remark: w.remark.clone(),
        merchant_account_id: w.merchant_account_id,
        account_data: w.account_data.clone(),
        audit_remark: w.audit_remark.clone(),
        payment_voucher: w.payment_voucher.clone(),
        withdraw_sn: w.withdraw_sn.clone(),
        withdraw_type: w.withdraw_type,
        pay_channel: w.pay_channel.clone(),
    }
}

/// 商户 → VO。
fn merchant_vo(m: &Merchant, with_type_text: bool) -> ShopMerchantVO {
    ShopMerchantVO {
        merchant_id: m.merchant_id,
        merchant_apply_id: m.merchant_apply_id,
        user_id: m.user_id,
        add_time: i64::from(m.add_time.unwrap_or(0)) * 1000,
        company_name: m.company_name.clone(),
        corporate_name: m.corporate_name.clone(),
        settlement_cycle: m.settlement_cycle,
        status: m.status,
        merchant_data: m.merchant_data.clone(),
        type_text: if with_type_text {
            // 类型 1 个人 2 企业（原 `MerchantType`，未知 code 返回空串）
            Some(
                match m.r#type {
                    Some(1) => "个人",
                    Some(2) => "企业",
                    _ => "",
                }
                .to_string(),
            )
        } else {
            None
        },
        admin: None,
    }
}

/// 评星（`ShopMapper` 的 `calculateShopStar` 片段）：
/// `shop_star_num < 100 / NULL / 0` → 4.8，否则取 `shop_star`。
fn calc_shop_star(shop: &Shop) -> Option<Decimal> {
    match shop.shop_star_num {
        None | Some(0) => Some(Decimal::new(48, 1)),
        Some(n) if n < 100 => Some(Decimal::new(48, 1)),
        _ => shop.shop_star,
    }
}

/// 递归构建经营类目树（对齐 hutool `TreeUtil.build`，根为 `parent_id = 0`）。
fn build_category_tree(cats: &[nextrade_core::entities::ShopProductCategory], parent: i32) -> Vec<ShopCategoryTreeNode> {
    cats.iter()
        .filter(|c| c.parent_id == parent)
        .map(|c| ShopCategoryTreeNode {
            category_id: c.category_id,
            parent_id: c.parent_id,
            category_name: c.category_name.clone(),
            category_pic: c.category_pic.clone(),
            is_store: 1,
            children: build_category_tree(cats, c.category_id),
        })
        .collect()
}

/// 渠道参数规范化（`AccountServiceImpl#normalizeChannel`）：大写白名单。
fn normalize_channel(channel: Option<&str>) -> Result<Option<String>> {
    let Some(raw) = channel.map(str::trim).filter(|s| !s.is_empty()) else {
        return Ok(None);
    };
    let up = raw.to_ascii_uppercase();
    match up.as_str() {
        "WECHAT" | "HUIFU" | "ALIPAY" => Ok(Some(up)),
        _ => Err(Error::Validation(
            "线上资金渠道仅支持 WECHAT、HUIFU、ALIPAY".into(),
        )),
    }
}

/// `yyyy-MM-dd` → 秒级时间戳，`end = true` 取当天 23:59:59。
/// 对齐原 Java `DateUtil.parse(text + " 00:00:00")`，按服务器本地时区解释。
fn day_range(text: Option<&str>, end: bool) -> Result<Option<i64>> {
    let Some(raw) = text.map(str::trim).filter(|s| !s.is_empty()) else {
        return Ok(None);
    };
    let date = chrono::NaiveDate::parse_from_str(raw, "%Y-%m-%d").map_err(|_| {
        Error::Validation(format!("时间格式错误：{raw}，应为 yyyy-MM-dd"))
    })?;
    let (h, m, s) = if end { (23, 59, 59) } else { (0, 0, 0) };
    let naive = date
        .and_hms_opt(h, m, s)
        .ok_or_else(|| Error::Validation(format!("时间格式错误：{raw}")))?;
    use chrono::TimeZone;
    let ts = chrono::Local
        .from_local_datetime(&naive)
        .single()
        .map(|dt| dt.timestamp())
        .unwrap_or_else(|| naive.and_utc().timestamp());
    Ok(Some(ts))
}

/// 取第一个非空白值（`keyword` 优先，其次 `shopTitle`）。
fn first_of<'a>(values: &[Option<&'a str>]) -> Option<&'a str> {
    values
        .iter()
        .filter_map(|v| *v)
        .map(str::trim)
        .find(|s| !s.is_empty())
}