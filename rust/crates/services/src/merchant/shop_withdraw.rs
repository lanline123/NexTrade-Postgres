//! 商家提现服务（`merchant::shop_withdraw`，8 个管理端端点）。
//!
//! 对应原 Java `com.nextrade.service.shop.impl.ShopWithdrawServiceImpl` 的：
//! `create` / `update` / `del` / `updateField` / `batch` / `uploadPayVoucher` /
//! `summaryList` / `getDualBalance`（另外 4 个端点 `list` / `detail` / `config` /
//! `audit` 已由 `merchant::shop` 实现）。
//!
//! # 已实现（与 Java 逐条对齐）
//!
//! - `create`：金额校验（>0 且最多两位小数）、提现类型 1/2、提现开关/限额、
//!   店铺可用余额、提现频率、提现单号 `yyyyMMddHHmmss + 5 位随机数`、状态 0、
//!   冻结金额语义（线下 = amount，线上 = 0）、账户信息 JSON。
//! - `update`：按状态更新（Java 复制属性后 `updateById`；该表无
//!   `shopWithdrawName/Pic/sortOrder/isShow` 列，仅 `status` 可落库）。
//! - `del` / `batch`：按主键/批量删除（Java `removeByIds`）。
//! - `updateField`：白名单字段更新；Java `GENERAL_FIELDS = {is_show, sort_order, status}`，
//!   但 `shop_withdraw` 仅有 `status` 列，故白名单收窄为 `status`（其余报
//!   `field 的值有误`，避免落到不存在的列）。
//! - `uploadPayVoucher`：线上提现禁止、状态必须为 4（待打款）、凭证必填，
//!   回填凭证并置状态 3（完成）。
//! - `summaryList`：`shop` LEFT JOIN `shop_withdraw` 聚合，支持店铺/时间/提现时间过滤，
//!   计算字段排序白名单，状态文案回填。
//! - `dualBalance`：线下余额 + 线上渠道余额结构。
//!
//! # 未迁移 / 偏差（诚实声明）
//!
//! 1. **多租户店铺上下文**：Java 用请求头 `X-Shop-Id`（`HeaderUtils.getShopId()`）；
//!    Rust JWT 无 shop_id，`create` / `dualBalance` 通过可选查询参数 `?shopId=`
//!    传入，缺省按 0 处理（此时 `create` 报「店铺可用余额不足」）。
//! 2. **线上提现（withdrawType=2）**：依赖分账子商户（`SubMerchantService`）、
//!    渠道余额查询（`ProfitSharingClientRegistry`）与渠道出金——均未迁移，
//!    本实现返回明确业务错误「线上提现依赖分账渠道（未迁移），暂不支持线上提现」。
//! 3. **账户/钱包联动**：`accountService.addWithDrawLog/completeWithDrawLog`、
//!    `recordRateService.saveRecordRate`、`statementService` 属资金/账务域，未迁移；
//!    只落 `shop_withdraw` 记录并记录 warn 日志。
//! 4. **后台站内信**：`notifyPlatformWithdrawal`（message 域）未迁移。
//! 5. 提现设置读取假设 `config` 扁平键：`withdrawal_enabled` / `min_amount` /
//!    `max_amount` / `withdrawal_frequency_unit` / `withdrawal_frequency_count`
//!    （与 Java `WithdrawalSettingsVO` 语义一致，键名待保证金设置代理最终确认）。

use std::collections::HashMap;
use std::sync::Arc;

use chrono::{Datelike, Local, TimeZone};
use rust_decimal::Decimal;

use nextrade_core::models::merchant::shop_manage::{BatchDTO, UpdateFieldDTO};
use nextrade_core::models::merchant::shop_withdraw::{
    shop_status_text, validate_amount, DualBalanceVO, ShopWithdrawCreateDTO,
    ShopWithdrawPayVoucherDTO, ShopWithdrawSummaryQuery, ShopWithdrawSummaryVO,
    ShopWithdrawUpdateDTO,
};
use nextrade_core::types::PageResult;
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::Repositories;

/// 提现状态：待审核（Java `PENDING_REVIEW`）。
const STATUS_PENDING_REVIEW: i32 = 0;
/// 提现状态：完成（Java `AUDIT_PASS`）。
const STATUS_AUDIT_PASS: i32 = 3;
/// 提现状态：待打款（Java `STATUS_WAIT_PAYMENT`）。
const STATUS_WAIT_PAYMENT: i32 = 4;
/// 提现方式：线下。
const WITHDRAW_TYPE_OFFLINE: i32 = 1;

/// 提现设置（Java `WithdrawalSettingsVO` 的本切片所需字段）。
#[derive(Debug, Clone, Default)]
pub struct WithdrawalSettings {
    pub enabled: Option<i32>,
    pub min_amount: Option<Decimal>,
    pub max_amount: Option<Decimal>,
    pub frequency_unit: Option<i32>,
    pub frequency_count: Option<i64>,
}

pub struct ShopWithdrawService {
    repos: Arc<Repositories>,
}

impl ShopWithdrawService {
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    // ————————————————————— 创建 —————————————————————

    /// `POST /adminapi/merchant/shopWithdraw/create`：创建提现申请。
    ///
    /// `shop_id` 为 Java `HeaderUtils.getShopId()` 的替身（见模块头偏差 1）。
    pub async fn create(&self, shop_id: i32, dto: &ShopWithdrawCreateDTO) -> Result<()> {
        let amount = validate_amount(dto.amount).map_err(Error::Validation)?;
        let withdraw_type = dto.withdraw_type.unwrap_or(WITHDRAW_TYPE_OFFLINE);
        if withdraw_type != 1 && withdraw_type != 2 {
            return Err(Error::Validation("提现类型只能为1或2".into()));
        }
        let settings = self.withdrawal_settings().await?;
        if withdraw_type == 2 {
            // 线上提现：开关注解 + 渠道子商户/余额/出金均未迁移
            if settings.enabled != Some(1) {
                return Err(Error::biz_msg("平台设置的线上提现功能未启用"));
            }
            return Err(Error::biz_msg(
                "线上提现依赖分账渠道（未迁移），暂不支持线上提现",
            ));
        }
        if settings.enabled != Some(1) {
            return Err(Error::biz_msg("平台设置的提现功能未启用"));
        }
        let min = settings.min_amount.unwrap_or(Decimal::ZERO);
        let max = settings.max_amount.unwrap_or(Decimal::ZERO);
        if amount < min || (max > Decimal::ZERO && amount > max) {
            return Err(Error::biz_msg(format!(
                "平台设置的提现金额限制为:{}-{}",
                min, max
            )));
        }
        // 店铺可用余额校验
        let shop = self
            .repos
            .merchant
            .shop_withdraw
            .find_shop(shop_id)
            .await?;
        let Some(shop) = shop else {
            return Err(Error::biz_msg("店铺可用余额不足"));
        };
        if shop.shop_money.unwrap_or(Decimal::ZERO) < amount {
            return Err(Error::biz_msg("店铺可用余额不足"));
        }
        // 提现频率限制
        self.check_frequency(shop_id, &settings, withdraw_type)
            .await?;

        let now = nextrade_common::utils::now_seconds() as i32;
        let sn = self.generate_withdraw_sn();
        let remark = dto.remark.clone().unwrap_or_default();
        let account_data = dto
            .account_data
            .as_ref()
            .map(|v| serde_json::to_string(v).unwrap_or_else(|_| "{}".into()))
            .unwrap_or_else(|| "{}".to_string());
        let offline_frozen = if withdraw_type == 2 {
            Decimal::ZERO
        } else {
            amount
        };
        let id = self
            .repos
            .merchant
            .shop_withdraw
            .insert(
                shop_id,
                amount,
                STATUS_PENDING_REVIEW,
                now,
                &remark,
                dto.merchant_account_id,
                Some(account_data),
                &sn,
                withdraw_type,
                dto.pay_channel.clone(),
                offline_frozen,
            )
            .await?;
        // 资金/账务联动与站内信未迁移（见模块头偏差 3/4）
        tracing::info!(
            "创建店铺提现（本地记录）: id={id}, shop_id={shop_id}, amount={amount}, sn={sn}"
        );
        Ok(())
    }

    // ————————————————————— 更新 / 删除 —————————————————————

    /// `POST /adminapi/merchant/shopWithdraw/update`：更新提现记录（仅状态可落库）。
    pub async fn update(&self, dto: &ShopWithdrawUpdateDTO) -> Result<()> {
        let id = dto
            .shop_withdraw_id
            .ok_or_else(|| Error::Validation("商家账户表id不能为空".into()))?;
        let exists = self
            .repos
            .merchant
            .shop_withdraw
            .find_by_id(id)
            .await?;
        if exists.is_none() {
            return Err(Error::biz_msg("提现记录不存在"));
        }
        if let Some(status) = dto.status {
            self.repos
                .merchant
                .shop_withdraw
                .update_status(id, status)
                .await?;
        }
        Ok(())
    }

    /// `POST /adminapi/merchant/shopWithdraw/del`：删除提现记录。
    pub async fn del(&self, id: Option<i32>) -> Result<()> {
        let id = id.ok_or_else(|| Error::Validation("商家账户表id不能为空".into()))?;
        self.repos
            .merchant
            .shop_withdraw
            .delete_by_ids(&[id])
            .await?;
        Ok(())
    }

    /// `POST /adminapi/merchant/shopWithdraw/updateField`：白名单字段更新。
    pub async fn update_field(&self, dto: &UpdateFieldDTO) -> Result<()> {
        let id = dto
            .id
            .ok_or_else(|| Error::Validation("商家账户表id不能为空".into()))?;
        let field = dto
            .field
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| Error::Validation("field 不能为空".into()))?;
        // Java GENERAL_FIELDS = {is_show, sort_order, status}，但本表无 is_show/sort_order 列
        if field != "status" {
            return Err(Error::biz_msg("field 的值有误"));
        }
        let val = dto
            .val
            .as_deref()
            .ok_or_else(|| Error::Validation("field 的值有误".into()))?;
        val.trim()
            .parse::<i32>()
            .map_err(|_| Error::biz_msg("field 的值有误"))?;
        self.repos
            .merchant
            .shop_withdraw
            .update_field(id, field, val.trim())
            .await?;
        Ok(())
    }

    /// `POST /adminapi/merchant/shopWithdraw/batch`：批量操作（仅支持 `del`）。
    pub async fn batch(&self, dto: &BatchDTO) -> Result<()> {
        if dto.r#type.as_deref() != Some("del") {
            return Ok(());
        }
        let ids = dto.ids.as_deref().unwrap_or_default();
        if ids.is_empty() {
            return Err(Error::Validation("商家账户表id不能为空".into()));
        }
        self.repos
            .merchant
            .shop_withdraw
            .delete_by_ids(ids)
            .await?;
        Ok(())
    }

    /// `POST /adminapi/merchant/shopWithdraw/uploadPayVoucher`：上传打款凭证。
    pub async fn upload_pay_voucher(&self, dto: &ShopWithdrawPayVoucherDTO) -> Result<()> {
        let id = dto
            .id
            .ok_or_else(|| ValidationError("#id错误"))?;
        let detail = self
            .repos
            .merchant
            .shop_withdraw
            .find_by_id(id)
            .await?;
        let Some(detail) = detail else {
            return Err(ValidationError("提现记录不存在"));
        };
        if detail.withdraw_type == 2 {
            return Err(ValidationError(
                "线上提现以渠道结果为准，不支持人工打款凭证完成",
            ));
        }
        if detail.status != STATUS_WAIT_PAYMENT {
            return Err(ValidationError("状态错误"));
        }
        let voucher = dto
            .payment_voucher
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| ValidationError("请上传打款凭证"))?;
        self.repos
            .merchant
            .shop_withdraw
            .update_voucher(id, voucher, STATUS_AUDIT_PASS)
            .await?;
        // 账户余额完成联动：释放冻结金额并写入 shop_account_log 流水（对齐 Java completeWithDrawLog）
        if let Err(e) = self.repos.merchant.shop.complete_withdraw(&detail).await {
            tracing::warn!(
                "uploadPayVoucher: 提现 {id} 账户余额联动失败: {e}"
            );
        } else {
            tracing::info!(
                "uploadPayVoucher: 提现 {id} 已完成打款，冻结资金已成功核销"
            );
        }
        Ok(())
    }

    // ————————————————————— 汇总 / 双余额 —————————————————————

    /// `GET /adminapi/merchant/shopWithdraw/summaryList`：店铺提现汇总列表。
    pub async fn summary_list(
        &self,
        q: &ShopWithdrawSummaryQuery,
    ) -> Result<PageResult<ShopWithdrawSummaryVO>> {
        let page = q.page_query();
        let (total, rows) = self
            .repos
            .merchant
            .shop_withdraw
            .paginate_summary(q, &page)
            .await?;
        let list = rows
            .into_iter()
            .map(|r| ShopWithdrawSummaryVO {
                shop_id: r.shop_id,
                shop_title: r.shop_title,
                shop_logo: r.shop_logo,
                status: r.status,
                status_text: Some(shop_status_text(r.status)),
                shop_money: r.shop_money,
                withdrawn_amount: r.withdrawn_amount,
                pending_withdraw_amount: r.pending_withdraw_amount,
                add_time: r.add_time,
            })
            .collect();
        Ok(PageResult::new(total, list, &page))
    }

    /// `GET /adminapi/merchant/shopWithdraw/dualBalance`：双余额查询。
    ///
    /// 线上三渠道（微信/汇付/支付宝）余额依赖分账子商户与渠道查单，均未迁移，
    /// 返回空渠道集合（`profitSharingEnabled=false`），线下余额正常返回。
    pub async fn dual_balance(&self, shop_id: i32) -> Result<DualBalanceVO> {
        let shop_money = if shop_id > 0 {
            self.repos
                .merchant
                .shop_withdraw
                .find_shop(shop_id)
                .await?
                .and_then(|s| s.shop_money)
                .unwrap_or(Decimal::ZERO)
        } else {
            Decimal::ZERO
        };
        let mut channel_statuses = std::collections::BTreeMap::new();
        channel_statuses.insert("WECHAT".to_string(), "UNBOUND".to_string());
        channel_statuses.insert("HUIFU".to_string(), "UNBOUND".to_string());
        channel_statuses.insert("ALIPAY".to_string(), "UNAVAILABLE".to_string());

        Ok(DualBalanceVO {
            shop_money,
            profit_sharing_enabled: false,
            channel_balances: Default::default(),
            channel_statuses,
            wechat_balance: None,
            online_balance: None,
            online_channel: None,
        })
    }

    // ————————————————————— 内部辅助 —————————————————————

    async fn withdrawal_settings(&self) -> Result<WithdrawalSettings> {
        let map = self
            .repos
            .merchant
            .shop_withdraw
            .config_values(&[
                "withdrawal_enabled",
                "min_amount",
                "max_amount",
                "withdrawal_frequency_unit",
                "withdrawal_frequency_count",
            ])
            .await?;
        Ok(WithdrawalSettings {
            enabled: cfg_int(&map, "withdrawal_enabled").map(|v| v as i32),
            min_amount: cfg_dec(&map, "min_amount"),
            max_amount: cfg_dec(&map, "max_amount"),
            frequency_unit: cfg_int(&map, "withdrawal_frequency_unit").map(|v| v as i32),
            frequency_count: cfg_int(&map, "withdrawal_frequency_count"),
        })
    }

    async fn check_frequency(
        &self,
        shop_id: i32,
        settings: &WithdrawalSettings,
        withdraw_type: i32,
    ) -> Result<()> {
        let (Some(unit), Some(max_count)) = (settings.frequency_unit, settings.frequency_count)
        else {
            return Ok(());
        };
        if unit == 0 || max_count <= 0 {
            return Ok(());
        }
        let Some((start_time, period_desc)) = frequency_period_start(unit) else {
            return Err(ValidationError("提现频率参数错误"));
        };
        let count = self
            .repos
            .merchant
            .shop_withdraw
            .count_since(shop_id, start_time, withdraw_type)
            .await?;
        if count >= max_count {
            return Err(ValidationError(format!(
                "平台设置的{period_desc}提现次数限制为:{max_count}"
            )));
        }
        Ok(())
    }

    /// 提现单号：`yyyyMMddHHmmss` + 5 位数字（Java `snPrefix + snSuffix`）。
    fn generate_withdraw_sn(&self) -> String {
        let prefix = Local::now().format("%Y%m%d%H%M%S").to_string();
        let suffix = nextrade_common::utils::now_millis().rem_euclid(100_000);
        format!("{prefix}{suffix:05}")
    }
}

// ————————————————————— 纯函数（可单测） —————————————————————

/// 业务错误（Java `GlobalException(msg)` 不带业务码 → 本实现用 `Error::Validation`
/// 与 `Error::biz_msg` 的组合保持文案逐字）。
#[allow(non_snake_case)]
fn ValidationError(msg: impl Into<String>) -> Error {
    Error::Validation(msg.into())
}

/// 频率周期起点：1 每日 2 每月 3 每年（Java `DateUtil.beginOfDay/Month/Year`）。
#[must_use]
pub fn frequency_period_start(unit: i32) -> Option<(i32, &'static str)> {
    let now = Local::now();
    let dt = match unit {
        1 => now.date_naive().and_hms_opt(0, 0, 0)?,
        2 => now.date_naive().with_day(1)?.and_hms_opt(0, 0, 0)?,
        3 => chrono::NaiveDate::from_ymd_opt(now.year(), 1, 1)?.and_hms_opt(0, 0, 0)?,
        _ => return None,
    };
    let ts = Local.from_local_datetime(&dt).earliest()?.timestamp() as i32;
    let desc = match unit {
        1 => "每日",
        2 => "每月",
        _ => "每年",
    };
    Some((ts, desc))
}

fn cfg_int(map: &HashMap<String, Option<String>>, key: &str) -> Option<i64> {
    map.get(key)
        .and_then(|v| v.as_deref())
        .and_then(|v| v.trim().parse::<i64>().ok())
}

fn cfg_dec(map: &HashMap<String, Option<String>>, key: &str) -> Option<Decimal> {
    map.get(key)
        .and_then(|v| v.as_deref())
        .and_then(|v| v.trim().parse::<Decimal>().ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frequency_period_start_matches_java_units() {
        let (day, day_desc) = frequency_period_start(1).unwrap();
        assert_eq!(day_desc, "每日");
        let now = nextrade_common::utils::now_seconds() as i32;
        assert!(day <= now);
        let (month, month_desc) = frequency_period_start(2).unwrap();
        assert_eq!(month_desc, "每月");
        assert!(month <= day);
        let (year, year_desc) = frequency_period_start(3).unwrap();
        assert_eq!(year_desc, "每年");
        assert!(year <= month);
        assert!(frequency_period_start(9).is_none());
    }

    #[test]
    fn config_parsing_helpers() {
        let mut map = HashMap::new();
        map.insert("withdrawal_enabled".to_string(), Some("1".to_string()));
        map.insert("min_amount".to_string(), Some("10.00".to_string()));
        map.insert("withdrawal_frequency_count".to_string(), Some("3".to_string()));
        map.insert("blank".to_string(), Some("  ".to_string()));
        assert_eq!(cfg_int(&map, "withdrawal_enabled"), Some(1));
        assert_eq!(
            cfg_dec(&map, "min_amount"),
            Some(Decimal::new(1000, 2))
        );
        assert_eq!(cfg_int(&map, "withdrawal_frequency_count"), Some(3));
        assert_eq!(cfg_int(&map, "missing"), None);
        assert_eq!(cfg_int(&map, "blank"), None);
        assert_eq!(cfg_dec(&map, "missing"), None);
    }
}
