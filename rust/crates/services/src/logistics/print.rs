//! 打印机管理域服务（logistics::print）。
//!
//! 对应原 Java `com.nextrade.service.print.impl.PrintServiceImpl`（600 行）
//! + `PrintRoutingServiceImpl#resolve` + `PrintPosDeviceServiceImpl#replace`
//! / `#bindingsByPrintIds` + `PrintConfigServiceImpl#initConfig`
//! / `#updatePaperSizeByPrintId` + `BaseServiceImpl#updateField`。
//!
//! # 端点 → 方法
//!
//! | 端点（挂载于 `/adminapi`） | 原 Java | 方法 |
//! |---------------------------|---------|------|
//! | `GET  /print/print/list` | `PrintController.list` | [`PrintService::list`] |
//! | `GET  /print/print/detail` | `PrintController.detail` | [`PrintService::detail`] |
//! | `POST /print/print/create` | `PrintController.create` | [`PrintService::create`] |
//! | `POST /print/print/update` | `PrintController.update` | [`PrintService::update`] |
//! | `POST /print/print/updateField` | `PrintController.updateField` | [`PrintService::update_field`] |
//! | `POST /print/print/del` | `PrintController.del` | [`PrintService::delete`] |
//! | `POST /print/print/hasEnabled` | `PrintController.hasEnabled` | [`PrintService::has_enabled`] |
//! | `GET  /print/print/devicePrinters` | `PrintController.devicePrinters` | [`PrintService::device_printers`] |
//!
//! # 语义要点（逐条对照 Java 源）
//!
//! - `list`：`WHERE shop_id = ?` 恒成立；过滤 `print_name` / `print_sn` /
//!   `status` / `platform` / `device_code`（`print_pos_device` 子查询）/
//!   `keyword`（名称或 SN，OR 模糊）。排序走 `BaseServiceImpl#buildSortOrder`
//!   —— 空 `sortField` 视为主键、`desc` 或空 `sortOrder` 一律降序，
//!   因此**默认是 `print_id DESC`**。列表回填 `paper_size`
//!   （`print_config`，缺省 58）与 `device_code`（`print_pos_device`）。
//! - `detail`：`WHERE print_id AND shop_id`，不存在抛 `打印机不存在`；
//!   `printSn` / `printKey` / `thirdKey` 经 `maskMiddleHalf` 脱敏，
//!   `paper_size` 取该打印机第一条配置（缺省 58）。
//! - `create`：必填校验 → 按平台归一（缺省飞鹅云）→ 落库
//!   （`status` 缺省 2 停用、`voiceEnabled` 缺省 1、`voiceContentType`
//!   缺省 0、`isDefault` 仅 `1` 视为默认、`printNumber` 固定 1）→
//!   `isDefault = 1` 时清除本店其它默认 → 初始化默认小票配置
//!   （已有配置则跳过）→ 重置设备绑定。
//! - `update`：先按 `print_id + shop_id` 取旧行（不存在抛 `打印机不存在`）
//!   → 脱敏回填（前端回传 `12*****890` 时沿用 DB 明文）→ 整行更新
//!   （Java `updateById` 的 `NOT_NULL` 语义：未传列保留旧值）→
//!   `paperSize` 传值时同步 `print_config` → `deviceCode` 非 `None` 时重置绑定
//!   （`Some("")` 表示解绑）。
//! - `updateField`：字段白名单 [`PRINT_FIELDS`]（`status` / `is_default`），
//!   黑名单外抛 `field 的值有误`；`isDefault` 置 1 前先清本店其它默认。
//! - `del`：物理删除 `print` 行 + 该打印机全部 `print_config` + 全部设备绑定。
//! - `hasEnabled`：本店是否存在 `status = 1` 的打印机。
//! - `devicePrinters`：复用 `PrintRoutingPolicy#select`（见
//!   [`crate::logistics::print_config::select_printers`]）：
//!   明确设备码 → 只取该设备绑定的启用打印机；否则默认打印机优先、
//!   再回退首台启用打印机。
//!
//! # 与 Java 的已知差异（`uncertain_points`）
//!
//! 1. **云平台适配器未迁移**：Java `create`/`update` 在飞鹅云 / 商米云
//!    （`FeieyunApiClient`、`SunmiPrintAdapter`）上先做云端增删改，失败即
//!    回滚；Java `del` 对商米云先云端解绑。本仓 `crates/infrastructure`
//!    尚无打印平台客户端，故这里只做**本地落库**（云端动作缺失，
//!    仅打印 `tracing::warn!`）。`create`/`update` 不因云端失败而失败。
//! 2. **设备码校验占位桩未复刻**：Java `PrintPosDeviceServiceImpl#replace`
//!    里 `boolean enabled = false; if (!enabled) throw ...` 是设备主数据表
//!    未迁移留下的占位桩（任何非空 `deviceCode` 都会抛
//!    「设备码不存在或已停用」并回滚）。本实现按真实意图写绑定，见
//!    `infrastructure::db::repo::logistics::print::PrintRepository#replace_binding`。
//! 3. **事务边界**：Java `create`/`update`/`del` 带 `@Transactional`；
//!    本服务的多表写入（print / print_config / print_pos_device）按顺序
//!    执行，**未**包在单个 DB 事务里（仓储方法各自持有连接池）。
//! 4. **多租户**：Java 用请求头 `HeaderUtils.getShopId()`；Rust JWT 尚未
//!    签发 `shop_id`，由路由层传 [`DEFAULT_SHOP_ID`]（平台 `0`）。
//! 5. `updateField` 的通用实现（Java `BaseServiceImpl#updateField`）**不**
//!    按店铺过滤，本实现保持一致（仅由 `update_field` 白名单约束列名）。
//! 6. 打印日志 `PrintLogService` 的状态机（`createPending` / `markPushed` /
//!    `markCallback`）不在本 feature 范围内（日志列表见
//!    `logistics::print_log`；MQ 消费端未迁移）。
//!
//! # 金额
//!
//! 本域不涉及任何金额列，故无 `rust_decimal::Decimal`。

use std::collections::HashMap;
use std::sync::Arc;

use hmac::{Hmac, Mac};
use sha2::Sha256;

use nextrade_core::entities::Print;
use nextrade_core::models::logistics::print::{
    is_default_flag, mask_middle_half, normalize_field, normalize_paper_size,
    normalize_platform, platform_desc, status_desc, status_or_disabled, validate_add,
    validate_update, voice_content_type_or_default, voice_enabled_or_default, PrintAddDTO,
    PrintDetailVO, PrintPageQuery, PrintUpdateDTO, PrintVO, UpdateFieldDTO,
    MSG_FIELD_NAME_REQUIRED, MSG_FIELD_VALUE_REQUIRED, MSG_INVALID_FIELD_VALUE,
    MSG_OPERATE_ID_REQUIRED, MSG_PRINTER_NOT_FOUND, PLATFORM_SUNMI,
};
use nextrade_core::models::logistics::print_config::{
    default_receipt_template, format_time, paper_size_code, paper_size_desc, CONFIG_TYPE_RECEIPT,
    DEFAULT_SHOP_ID, PAPER_SIZE_58,
};
use nextrade_core::types::{PageQuery, PageResult};
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::logistics::print::{PrintFilters, PrintRepository};
use nextrade_infrastructure::db::Repositories;

use crate::logistics::print_config::select_printers;

/// 管理端每页条数缺省值（Java `BasePage#getSize()` 默认 15）。
pub const DEFAULT_PAGE_SIZE: u64 = 15;

/// 管理端每页条数上限（Java `BasePage#getSize()` 上限 100）。
pub const MAX_PAGE_SIZE: u64 = 100;

/// 打印机管理域服务。
pub struct PrintService {
    repos: Arc<Repositories>,
}

impl PrintService {
    /// 构造（与同域 `print_config` / 其它 feature 的 `new(Arc<Repositories>)` 一致）。
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    /// 本片仓储。
    #[must_use]
    pub fn repo(&self) -> &PrintRepository {
        &self.repos.logistics.print
    }

    /// 路由层默认使用的店铺 ID（平台 `0`，见模块头 `uncertain_points` 第 4 条）。
    #[must_use]
    pub const fn default_shop_id() -> i32 {
        DEFAULT_SHOP_ID
    }

    // ————————————————————— 查询 —————————————————————

    /// 打印机分页列表（对应 Java `PrintServiceImpl#list`）。
    ///
    /// 排序字段白名单 / 回退与 Java `BaseServiceImpl#buildSortOrder` 一致，
    /// 见 `infrastructure::db::repo::logistics::print::resolve_sort_column`。
    ///
    /// # Errors
    /// 数据库执行失败时返回 `Error::Database`。
    pub async fn list(&self, q: &PrintPageQuery, shop_id: i32) -> Result<PageResult<PrintVO>> {
        let page = PageQuery {
            page: q.page.unwrap_or(1),
            page_size: q
                .page_size
                .unwrap_or(DEFAULT_PAGE_SIZE)
                .min(MAX_PAGE_SIZE),
        };
        let filters = PrintFilters {
            print_name: q.print_name.as_deref(),
            print_sn: q.print_sn.as_deref(),
            status: q.status,
            platform: q.platform,
            device_code: q.device_code.as_deref(),
            keyword: q.keyword.as_deref(),
        };
        let (total, rows) = self
            .repo()
            .paginate(
                shop_id,
                &filters,
                q.sort_field.as_deref(),
                q.sort_order.as_deref(),
                &page,
            )
            .await?;

        let print_ids: Vec<i32> = rows.iter().map(|p| p.print_id).collect();
        let papers = self.paper_sizes(&print_ids).await?;
        let bindings = self.bindings(shop_id, &print_ids).await?;

        let list: Vec<PrintVO> = rows
            .iter()
            .map(|p| {
                // Java `paperByPrintId.getOrDefault(printId, SIZE_58)`：
                // 列表**一定**回填 paperSize，无配置时兜底 58mm
                let paper = papers
                    .get(&p.print_id)
                    .copied()
                    .map_or(PAPER_SIZE_58, paper_size_code);
                to_vo(
                    p,
                    Some(paper),
                    bindings.get(&p.print_id).cloned(),
                )
            })
            .collect();
        Ok(PageResult::new(total, list, &page))
    }

    /// 打印机详情（对应 Java `PrintServiceImpl#detail`）。
    ///
    /// 不存在（或不属于当前店铺）→ `打印机不存在`（Java `GlobalException`）。
    ///
    /// # Errors
    /// - 打印机不存在 → `Error::Biz { code: 500, msg: 打印机不存在 }`
    /// - 数据库执行失败 → `Error::Database`
    pub async fn detail(&self, print_id: i32, shop_id: i32) -> Result<PrintDetailVO> {
        let p = self
            .repo()
            .find_by_id_and_shop(print_id, shop_id)
            .await?
            .ok_or_else(|| Error::biz(500, MSG_PRINTER_NOT_FOUND))?;

        // Java：从 print_config 取第一条配置的 paper_size，null → 58
        let papers = self.paper_sizes(&[print_id]).await?;
        let paper = papers
            .get(&print_id)
            .copied()
            .map_or(PAPER_SIZE_58, paper_size_code);
        // Java：bindingsByPrintIds(shopId, List.of(id)).get(id)
        let bindings = self.bindings(shop_id, &[print_id]).await?;
        let device_code = bindings.get(&print_id).cloned();
        Ok(to_detail_vo(&p, paper, device_code))
    }

    /// 本店是否存在启用的打印机
    /// （对应 Java `PrintServiceImpl#hasEnabled`）。
    ///
    /// # Errors
    /// 数据库执行失败时返回 `Error::Database`。
    pub async fn has_enabled(&self, shop_id: i32) -> Result<bool> {
        self.repo().has_enabled(shop_id).await
    }

    /// 查询收银台设备可用的打印机
    /// （对应 Java `PrintServiceImpl#devicePrinters` +
    /// `PrintRoutingServiceImpl#resolve` + `PrintRoutingPolicy#select`）。
    ///
    /// 设备码空白与 `None` 等价（Java `StrUtil.isBlank(deviceCode) ? null`）。
    /// ⚠️ 该接口的 VO **不回填** `paperSize` / `paperSizeDesc`
    /// （Java 同一 VO 的另一条构造路径），故两者为 `null`。
    ///
    /// # Errors
    /// 数据库执行失败时返回 `Error::Database`。
    pub async fn device_printers(
        &self,
        device_code: Option<&str>,
        shop_id: i32,
    ) -> Result<Vec<PrintVO>> {
        let code = device_code
            .map(str::trim)
            .filter(|c| !c.is_empty());
        let enabled = self.repo().find_enabled(shop_id).await?;
        let print_ids: Vec<i32> = enabled.iter().map(|p| p.print_id).collect();
        let bindings = self.bindings(shop_id, &print_ids).await?;
        let default_id = enabled
            .iter()
            .find(|p| p.is_default == 1)
            .map(|p| p.print_id);
        let selected = select_printers(code, &enabled, &bindings, default_id);
        Ok(selected
            .iter()
            .map(|p| to_vo(p, None, bindings.get(&p.print_id).cloned()))
            .collect())
    }

    // ————————————————————— 写操作 —————————————————————

    /// 新增打印机（对应 Java `PrintServiceImpl#create`）。
    ///
    /// Java 返回值是 `save(print)` 的布尔值（成功即 `true`）。
    /// ⚠️ 云平台新增（飞鹅云 `Open_printerAddlist` / 商米云 `bindShop`）未迁移，
    /// 本实现直接本地落库，见模块头 `uncertain_points` 第 1/2 条。
    ///
    /// # Errors
    /// - 入参校验失败 → `Error::Validation(<Java 注解文案>)`
    /// - 数据库执行失败 → `Error::Database`
    pub async fn create(&self, dto: &PrintAddDTO, shop_id: i32) -> Result<bool> {
        validate_add(dto).map_err(Error::Validation)?;

        let now = now_secs_i32();
        let is_default = is_default_flag(dto.is_default);
        let print = Print {
            print_id: 0,
            print_name: dto.print_name.clone(),
            print_sn: dto.print_sn.clone(),
            print_key: dto.print_key.clone(),
            third_account: dto.third_account.clone(),
            third_key: dto.third_key.clone(),
            // Java 固定 printNumber = 1
            print_number: Some(1),
            platform: Some(normalize_platform(dto.platform)),
            shop_id: Some(shop_id),
            status: Some(status_or_disabled(dto.status)),
            add_time: Some(now),
            update_time: Some(now),
            delete_time: None,
            auto_print: dto.auto_print,
            voice_enabled: voice_enabled_or_default(dto.voice_enabled),
            voice_content_type: voice_content_type_or_default(dto.voice_content_type),
            is_default,
        };

        // Java：clearOtherDefault(null, isDefault) —— 全店先清 0
        if is_default == 1 {
            self.repo().clear_other_default(shop_id, None).await?;
        }

        let print_id = self.repo().insert(&print).await?;

        // Java：initConfig 失败只记 warn，不影响 create 结果
        let paper = normalize_paper_size(dto.paper_size);
        if let Err(e) = self.init_config(print_id, paper, now).await {
            tracing::warn!("打印机配置初始化失败，打印机ID：{print_id}，原因：{e}");
        }

        // Java：bindingService.replace(shopId, printId, deviceCode)
        self.repo()
            .replace_binding(shop_id, print_id, dto.device_code.as_deref(), i64::from(now))
            .await?;

        tracing::info!(
            "新增打印机成功: print_id={print_id}, platform={}, shop_id={shop_id}",
            print.platform.unwrap_or_default()
        );
        Ok(true)
    }

    /// 编辑打印机（对应 Java `PrintServiceImpl#update`）。
    ///
    /// 脱敏回填：前端把详情接口的 `printSn` / `printKey` / `thirdKey`
    /// 原样回传（`12*****890`）时沿用库中明文，见
    /// [`nextrade_core::models::logistics::print::pick_secret_for_update`]。
    ///
    /// # Errors
    /// - 入参校验失败 → `Error::Validation(<Java 注解文案>)`
    /// - 打印机不存在 / 不属于本店 → `Error::biz(500, 打印机不存在)`
    /// - 数据库执行失败 → `Error::Database`
    pub async fn update(&self, dto: &PrintUpdateDTO, shop_id: i32) -> Result<bool> {
        validate_update(dto).map_err(Error::Validation)?;
        // validate_update 已保证 print_id 存在
        let print_id = dto.print_id.unwrap_or_default();

        let old = self
            .repo()
            .find_by_id_and_shop(print_id, shop_id)
            .await?
            .ok_or_else(|| Error::biz(500, MSG_PRINTER_NOT_FOUND))?;

        let now = now_secs_i32();
        let target = Print {
            print_id,
            // Java `updateById` NOT_NULL：必填项直接覆盖
            print_name: dto.print_name.clone().or_else(|| old.print_name.clone()),
            // 三个密钥列走「脱敏回填」逻辑
            print_sn: nextrade_core::models::logistics::print::pick_secret_for_update(
                old.print_sn.as_deref(),
                dto.print_sn.as_deref(),
            ),
            print_key: nextrade_core::models::logistics::print::pick_secret_for_update(
                old.print_key.as_deref(),
                dto.print_key.as_deref(),
            ),
            third_account: dto.third_account.clone().or_else(|| old.third_account.clone()),
            third_key: nextrade_core::models::logistics::print::pick_secret_for_update(
                old.third_key.as_deref(),
                dto.third_key.as_deref(),
            ),
            // Java 固定 printNumber = 1
            print_number: Some(1),
            // Java update 不写 platform / shop_id（NOT_NULL 且未赋值）→ 保留旧值
            platform: old.platform,
            shop_id: old.shop_id,
            status: dto.status.or(old.status),
            add_time: old.add_time,
            update_time: Some(now),
            delete_time: old.delete_time,
            auto_print: dto.auto_print.or(old.auto_print),
            // 语音两列：传值才覆盖，null 保留原值（旧前端兼容）
            voice_enabled: dto.voice_enabled.unwrap_or(old.voice_enabled),
            voice_content_type: dto.voice_content_type.unwrap_or(old.voice_content_type),
            // isDefault：传值才覆盖
            is_default: dto.is_default.map_or(old.is_default, |v| is_default_flag(Some(v))),
        };

        // Java：clearOtherDefault(printId, isDefault) 仅在置 1 时生效
        if dto.is_default == Some(1) {
            self.repo()
                .clear_other_default(shop_id, Some(print_id))
                .await?;
        }

        let updated = self.repo().update(&target).await?;
        if updated {
            // 纸张尺寸：传值时同步 print_config（旧前端不传则保留历史值）
            if let Some(paper) = dto.paper_size {
                self.repo()
                    .update_paper_size(print_id, normalize_paper_size(Some(paper)), now)
                    .await?;
            }
            // 设备码：`Some("")` 表示解绑；`None` 表示不动
            if dto.device_code.is_some() {
                self.repo()
                    .replace_binding(
                        shop_id,
                        print_id,
                        dto.device_code.as_deref(),
                        i64::from(now),
                    )
                    .await?;
            }
        }
        tracing::info!("编辑打印机: print_id={print_id}, 影响 updated={updated}");
        Ok(updated)
    }

    /// 更新单个字段（对应 Java `PrintServiceImpl#updateField`
    /// → `BaseServiceImpl#updateField`，白名单 `PRINT_FIELDS`）。
    ///
    /// Java 的前置动作：`field == "isDefault" && val == "1"` 时先清本店
    /// 其它默认打印机（比较的是**属性名**，不是列名，逐字复刻）。
    ///
    /// # Errors
    /// - 缺 `id` / `field` / `val` → `Error::Validation(<Java 注解文案>)`
    /// - 字段不在白名单 → `Error::Validation(field 的值有误)`
    /// - 数据库执行失败 → `Error::Database`
    pub async fn update_field(&self, dto: &UpdateFieldDTO, shop_id: i32) -> Result<bool> {
        let id = dto
            .id
            .ok_or_else(|| Error::Validation(MSG_OPERATE_ID_REQUIRED.into()))?;
        let field = dto
            .field
            .as_deref()
            .ok_or_else(|| Error::Validation(MSG_FIELD_NAME_REQUIRED.into()))?;
        let val = dto
            .val
            .as_deref()
            .ok_or_else(|| Error::Validation(MSG_FIELD_VALUE_REQUIRED.into()))?;

        // Java 先比较属性名字面量 "isDefault"，再走列名归一
        if field == "isDefault" && val == "1" {
            self.repo().clear_other_default(shop_id, Some(id)).await?;
        }

        let column = normalize_field(field)
            .ok_or_else(|| Error::Validation(MSG_INVALID_FIELD_VALUE.into()))?;
        let updated = self.repo().update_field(id, column, val).await?;
        tracing::info!("更新打印机字段: print_id={id}, {column}={val}, updated={updated}");
        Ok(updated)
    }

    /// 删除打印机（对应 Java `PrintServiceImpl#del`）。
    ///
    /// 物理删除 `print` 行，并清理该打印机的全部 `print_config` 与
    /// `print_pos_device` 绑定（Java 的三条写操作）。
    ///
    /// ⚠️ 商米云在 Java 里会先 `adapter.unbind(print)`（失败则不删本地）；
    /// 适配器未迁移，本实现仅本地删除并记 warn，见模块头 `uncertain_points`。
    ///
    /// # Errors
    /// - 打印机不存在 / 不属于本店 → `Error::biz(500, 打印机不存在)`
    /// - 数据库执行失败 → `Error::Database`
    pub async fn delete(&self, print_id: i32, shop_id: i32) -> Result<()> {
        let p = self
            .repo()
            .find_by_id_and_shop(print_id, shop_id)
            .await?
            .ok_or_else(|| Error::biz(500, MSG_PRINTER_NOT_FOUND))?;

        // 商米云：先同步解绑云端（DC-02=A 强一致），失败不删本地
        if p.platform == Some(PLATFORM_SUNMI) {
            unbind_sunmi(&p).await?;
        }

        let deleted = self.repo().delete(print_id).await?;
        if !deleted {
            return Err(Error::biz(500, MSG_PRINTER_NOT_FOUND));
        }
        // Java：printConfigMapper.delete(eq(printId))
        self.repo().delete_configs(print_id).await?;
        // Java：bindingService.replace(shopId, id, null)
        self.repo().delete_bindings(shop_id, print_id).await?;
        tracing::info!("删除打印机: print_id={print_id}, shop_id={shop_id}");
        Ok(())
    }

    // ————————————————————— 内部 —————————————————————

    /// 批量读取纸张尺寸并去重
    /// （Java `Collectors.toMap(..., (existing, replacement) -> existing)`：
    /// 同一打印机多条配置取**先出现**的一条）。
    async fn paper_sizes(&self, print_ids: &[i32]) -> Result<HashMap<i32, i32>> {
        let rows = self.repo().find_paper_sizes(print_ids).await?;
        let mut map: HashMap<i32, i32> = HashMap::with_capacity(rows.len());
        for (print_id, paper_size) in rows {
            map.entry(print_id).or_insert(paper_size);
        }
        Ok(map)
    }

    /// 批量读取设备绑定并去重
    /// （Java `rows.forEach(row -> result.putIfAbsent(printId, deviceCode))`：
    /// 同一打印机多条绑定取**先出现**的一条）。
    async fn bindings(&self, shop_id: i32, print_ids: &[i32]) -> Result<HashMap<i32, String>> {
        let rows = self.repo().find_bindings(shop_id, print_ids).await?;
        let mut map: HashMap<i32, String> = HashMap::with_capacity(rows.len());
        for (print_id, device_code) in rows {
            map.entry(print_id).or_insert(device_code);
        }
        Ok(map)
    }

    /// 初始化默认购物小票配置（对应 Java `PrintConfigServiceImpl#initConfig`）：
    /// 已有配置 → 直接返回 `Ok(())`（Java 返回 `true`，不重复初始化）。
    async fn init_config(&self, print_id: i32, paper_size: i32, now_secs: i32) -> Result<()> {
        if self.repo().count_configs(print_id).await? > 0 {
            tracing::info!("打印机ID:{print_id}已存在配置，跳过初始化");
            return Ok(());
        }
        let template_json = serde_json::to_string(&default_receipt_template())
            .map_err(|e| Error::Internal(format!("初始化默认打印模板失败: {e}")))?;
        self.repo()
            .insert_default_config(
                print_id,
                &template_json,
                paper_size,
                now_secs,
                CONFIG_TYPE_RECEIPT,
            )
            .await?;
        Ok(())
    }
}

/// 商米云打印机解绑（对应 Java `SunmiPrintAdapter#unbind`）
async fn unbind_sunmi(print: &Print) -> Result<()> {
    let appid = print.third_account.as_deref().unwrap_or_default().trim();
    let appkey = print.third_key.as_deref().unwrap_or_default().trim();
    let sn = print.print_sn.as_deref().unwrap_or_default().trim();
    if appid.is_empty() || appkey.is_empty() || sn.is_empty() {
        return Ok(());
    }

    let shop_id = match print.shop_id {
        Some(s) if s > 0 => s,
        _ => 1, // Java SHOP_ID_FALLBACK = 1
    };
    let payload = serde_json::json!({
        "sn": sn,
        "shop_id": shop_id,
    });
    let body = payload.to_string();
    let timestamp = nextrade_common::utils::now_seconds().to_string();
    let nonce = format!("{:06}", rand::random::<u32>() % 900_000 + 100_000);

    let sign_str = format!("{body}{appid}{timestamp}{nonce}");
    let mut mac = Hmac::<Sha256>::new_from_slice(appkey.as_bytes())
        .map_err(|e| Error::biz_msg(format!("初始化 HMAC 失败: {e}")))?;
    mac.update(sign_str.as_bytes());
    let sign = hex::encode(mac.finalize().into_bytes());

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| Error::biz_msg(format!("创建 HTTP 客户端失败: {e}")))?;

    let resp = client
        .post("https://openapi.sunmi.com/v2/printer/open/open/device/unbindShop")
        .header("Source", "openapi")
        .header("Sunmi-Appid", appid)
        .header("Sunmi-Timestamp", &timestamp)
        .header("Sunmi-Nonce", &nonce)
        .header("Sunmi-Sign", &sign)
        .header("Content-Type", "application/json")
        .body(body)
        .send()
        .await
        .map_err(|e| Error::biz_msg(format!("请求商米云失败: {e}")))?;

    let text = resp
        .text()
        .await
        .map_err(|e| Error::biz_msg(format!("读取商米云响应失败: {e}")))?;

    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) {
        let code = v.get("code").and_then(|c| c.as_i64()).unwrap_or(-1);
        // 成功码为 1 或 10000，以及幂等错误码 10071701 / 10071704 / 10071707
        if code == 1 || code == 10000 || code == 10071701 || code == 10071704 || code == 10071707 {
            return Ok(());
        }
        let msg = v.get("msg").and_then(|m| m.as_str()).unwrap_or("未知错误");
        return Err(Error::biz_msg(format!("商米云解绑失败 ({code}): {msg}")));
    }

    Ok(())
}

/// 列表行映射（Java `new PrintPageVO(print)` + 回填 paperSize / deviceCode）。
///
/// `paper_size = None` 表示「该调用路径不回填」（`devicePrinters`），
/// 与 Java 同一 VO 的两条构造路径一致。
#[must_use]
fn to_vo(p: &Print, paper_size: Option<i32>, device_code: Option<String>) -> PrintVO {
    PrintVO {
        print_id: p.print_id,
        print_name: p.print_name.clone(),
        // 列表不脱敏（Java PrintPageVO 直接用实体字段）
        print_sn: p.print_sn.clone(),
        platform: p.platform,
        platform_text: platform_desc(p.platform).to_string(),
        status: p.status,
        status_text: status_desc(p.status).to_string(),
        paper_size: paper_size.map(paper_size_code),
        paper_size_desc: paper_size.map(|s| paper_size_desc(s).to_string()),
        // 实体两列 NOT NULL，无需 null 兜底（Java 是 Integer，兜底 1 / 0）
        voice_enabled: p.voice_enabled,
        voice_content_type: p.voice_content_type,
        device_code,
        is_default: p.is_default,
        update_time: format_time(p.update_time),
        add_time: format_time(p.add_time),
    }
}

/// 详情映射（Java `new PrintDetailVO(print)` + 回填 paperSize / deviceCode）。
///
/// 三个密钥列按 Java `StringUtils#maskMiddleHalf` 脱敏：`printSn` /
/// `printKey` / `thirdKey`；`thirdAccount` 明文。
#[must_use]
fn to_detail_vo(p: &Print, paper_size: i32, device_code: Option<String>) -> PrintDetailVO {
    PrintDetailVO {
        print_id: p.print_id,
        print_name: p.print_name.clone(),
        print_sn: mask_middle_half(p.print_sn.as_deref()),
        print_key: mask_middle_half(p.print_key.as_deref()),
        third_account: p.third_account.clone(),
        third_key: mask_middle_half(p.third_key.as_deref()),
        print_number: p.print_number,
        platform: p.platform,
        platform_text: platform_desc(p.platform).to_string(),
        status: p.status,
        auto_print: p.auto_print,
        status_text: status_desc(p.status).to_string(),
        shop_id: p.shop_id,
        paper_size: paper_size_code(paper_size),
        paper_size_desc: paper_size_desc(paper_size).to_string(),
        voice_enabled: p.voice_enabled,
        device_code,
        is_default: p.is_default,
        add_time: format_time(p.add_time),
        update_time: format_time(p.update_time),
    }
}

/// 当前 Unix 秒（等价 Java `System.currentTimeMillis() / 1000` 存入 `int` 列）。
fn now_secs_i32() -> i32 {
    i32::try_from(nextrade_common::utils::now_seconds()).unwrap_or(i32::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn printer(id: i32) -> Print {
        Print {
            print_id: id,
            print_name: Some("前台小票机".into()),
            print_sn: Some("1234567890".into()),
            print_key: Some("abcdef".into()),
            third_account: Some("feie-acc".into()),
            third_key: Some("third-key".into()),
            print_number: Some(1),
            platform: Some(1),
            shop_id: Some(0),
            status: Some(1),
            add_time: Some(1_700_000_000),
            update_time: Some(1_700_000_100),
            auto_print: Some(1),
            voice_enabled: 1,
            voice_content_type: 0,
            is_default: 1,
            ..Default::default()
        }
    }

    /// 默认店铺 ID 是平台 0（Java `HeaderUtils.getShopId()` 在 `admin` 下取 0）。
    #[test]
    fn default_shop_id_is_platform() {
        assert_eq!(PrintService::default_shop_id(), 0);
        assert_eq!(PrintService::default_shop_id(), DEFAULT_SHOP_ID);
    }

    /// 列表行映射：平台/状态文案、缺省纸张、时间格式化逐字段对齐 Java。
    #[test]
    fn list_vo_maps_fields_like_java() {
        let vo = to_vo(&printer(7), Some(80), Some("POS-A".into()));
        assert_eq!(vo.print_id, 7);
        assert_eq!(vo.print_name.as_deref(), Some("前台小票机"));
        // 列表**不**脱敏
        assert_eq!(vo.print_sn.as_deref(), Some("1234567890"));
        assert_eq!(vo.platform_text, "飞鹅云");
        assert_eq!(vo.status_text, "启用");
        assert_eq!(vo.paper_size, Some(80));
        assert_eq!(vo.paper_size_desc.as_deref(), Some("80mm"));
        assert_eq!(vo.device_code.as_deref(), Some("POS-A"));
        assert_eq!(vo.is_default, 1);
        assert_eq!(vo.add_time, "2023-11-15 06:13:20");
        assert_eq!(vo.update_time, "2023-11-15 06:15:00");
    }

    /// `devicePrinters` 路径不回填纸张尺寸 → `null`（Java 同 VO 另一条构造）。
    #[test]
    fn device_printers_vo_leaves_paper_size_null() {
        let vo = to_vo(&printer(7), None, None);
        assert!(vo.paper_size.is_none());
        assert!(vo.paper_size_desc.is_none());
        assert!(vo.device_code.is_none());
        // 其它字段照常
        assert_eq!(vo.status_text, "启用");
    }

    /// 列表行容许未知状态 / 平台（VO 兜底文案，不 panic）。
    #[test]
    fn list_vo_tolerates_unknown_enum_codes() {
        let mut p = printer(3);
        p.platform = Some(99);
        p.status = None;
        let vo = to_vo(&p, Some(0), None);
        assert_eq!(vo.platform_text, "未知平台");
        assert_eq!(vo.status_text, "未知");
        // paper_size 0 归一为 58（Java PrintPaperSizeEnum#of）
        assert_eq!(vo.paper_size, Some(58));
        assert_eq!(vo.paper_size_desc.as_deref(), Some("58mm"));
    }

    /// 详情映射：三个密钥列脱敏，`thirdAccount` 明文、纸张缺省 58。
    #[test]
    fn detail_vo_masks_secrets_and_keeps_account() {
        let vo = to_detail_vo(&printer(7), 58, Some("POS-A".into()));
        assert_eq!(vo.print_sn.as_deref(), Some("12*****890"));
        // Java `StringUtils.maskMiddleHalf`：len=6 → start=(6-3)/2=1，替换 [1,4) 共 3 个字符
        assert_eq!(vo.print_key.as_deref(), Some("a***ef"));
        // third_key="third-key"(len=9) → start=(9-4)/2=2，替换 [2,6) 共 4 个字符
        assert_eq!(vo.third_key.as_deref(), Some("th****key"));
        assert_eq!(vo.third_account.as_deref(), Some("feie-acc"));
        assert_eq!(vo.paper_size, 58);
        assert_eq!(vo.paper_size_desc, "58mm");
        assert_eq!(vo.auto_print, Some(1));
        assert_eq!(vo.shop_id, Some(0));
        assert_eq!(vo.device_code.as_deref(), Some("POS-A"));
        assert_eq!(vo.add_time, "2023-11-15 06:13:20");
    }

    /// 详情映射：未知纸张值归一 58（Java `PrintPaperSizeEnum#of` 兜底）。
    #[test]
    fn detail_vo_normalizes_unknown_paper_size() {
        let vo = to_detail_vo(&printer(7), 0, None);
        assert_eq!(vo.paper_size, 58);
        assert_eq!(vo.paper_size_desc, "58mm");
    }

    /// 新增/编辑的默认值归一（与 Java 三元表达式逐条对齐）。
    #[test]
    fn write_defaults_match_java() {
        assert_eq!(status_or_disabled(None), 2);
        assert_eq!(normalize_platform(None), 1);
        assert_eq!(normalize_platform(Some(2)), 2);
        assert_eq!(normalize_platform(Some(99)), 1);
        assert_eq!(voice_enabled_or_default(None), 1);
        assert_eq!(voice_content_type_or_default(None), 0);
        assert_eq!(is_default_flag(Some(1)), 1);
        assert_eq!(is_default_flag(None), 0);
        assert_eq!(normalize_paper_size(None), 58);
    }

    /// `update_field` 的字段归一：属性名与列名都认，白名单外拒绝。
    #[test]
    fn update_field_accepts_property_and_column_names() {
        assert_eq!(normalize_field("status"), Some("status"));
        assert_eq!(normalize_field("isDefault"), Some("is_default"));
        assert_eq!(normalize_field("is_default"), Some("is_default"));
        assert_eq!(normalize_field("printName"), None);
        assert_eq!(MSG_INVALID_FIELD_VALUE, "field 的值有误");
    }
}
