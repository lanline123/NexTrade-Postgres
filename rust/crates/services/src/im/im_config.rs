//! IM 配置域服务（im::im_config）：读取 / 保存 IM 客服配置。
//!
//! 对应原 Java：
//! - `com.nextrade.adminapi.controller.im.IMConfigController`
//!   （`@RequestMapping("im/config/config")`，2 个端点）
//! - `com.nextrade.service.im.impl.ImConfigServiceImpl#detail` / `#create`
//! - `com.nextrade.common.utils.JsonUtil`（`im_config.data` 的 JSON 读写）
//! - `com.nextrade.common.utils.HeaderUtils#getShopId`（多租户过滤，
//!   在路由层解析后以 [`ImConfigScope`] 传入）
//!
//! **未迁移 / 已简化**：
//! - Java `ImConfigServiceImpl` 还继承 `BaseServiceImpl`，提供 `list` /
//!   `update` 等能力；本切片只实现契约内的 `detail` / `save` 两个端点，
//!   未迁移 `list`（分页查询）与 `update`（按主键整行更新），
//!   见返回值的 `uncertain_points`。
//! - `@NotBlank` 的 `code` 校验在 Rust 侧以显式校验复刻；
//!   `ConfigConstants._OVER_LENGTH`（长度 50）在 Java DTO 上**并未**挂
//!   `@Size`，故不额外限制（保持与原实现一致的宽松行为）。
//! - `im_config.data` 的 JSON 由 `serde_json` 读写。Java `JsonUtil.toJson`
//!   使用 `INDENT_OUTPUT` 美化输出，本实现产出紧凑 JSON；语义等价，
//!   仅落库空白字符不同。

use std::sync::Arc;

use nextrade_core::models::im::im_config::{ConfigCreateDTO, ConfigDataDTO, ImConfigScope};
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::im::im_config::ImConfigRepository;
use nextrade_infrastructure::db::Repositories;

/// `code` 为空白时的提示文案。
///
/// Java `ConfigCreateDTO.code` 上只有裸 `@NotBlank`（**无 message**），
/// 校验失败由 `GlobalExceptionHandler#handleMethodArgumentNotValidException`
/// 取出 `error.getDefaultMessage()`，即 Jakarta Bean Validation 的默认英文文案
/// `must not be blank`，再经 `translatePackage` 翻译。此处使用其默认英文文案，
/// 未编造中文翻译（翻译服务未迁移）。
///
/// 注意：Java `ConfigConstants._NOT_NULL = "im配置名称不能为空"` 在本切片
/// 的调用链上**从未被使用**（全仓库 grep 无引用），故不采用该文案。
const ERR_CODE_NOT_BLANK: &str = "must not be blank";

/// IM 配置域服务。
pub struct ImConfigService {
    repos: Arc<Repositories>,
}

impl ImConfigService {
    /// 构造服务。
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    /// 本片仓储。
    fn repo(&self) -> &ImConfigRepository {
        &self.repos.im.im_config
    }

    /// `GET /adminapi/im/config/config/detail?code=`：获取 IM 配置详情。
    ///
    /// 对应 Java `ImConfigServiceImpl#detail(String code, Integer shopId)`：
    /// - `code == null` → 直接返回 `null`（**不查库**）；
    /// - 否则按 `code` + （`shopId` 非空时）`shop_id` 查一行；
    /// - 命中则把 `data` 的 JSON 反序列化为 `ConfigDataDTO` 返回，
    ///   未命中或 `data` 为空/非法 JSON 一律返回 `null`。
    ///
    /// # Errors
    /// 仅可能返回数据库错误（查询失败）；「未找到」不是错误，而是 `Ok(None)`。
    pub async fn detail(
        &self,
        code: Option<&str>,
        scope: ImConfigScope,
    ) -> Result<Option<ConfigDataDTO>> {
        // Java：`if (code != null)`，空串同样进查询分支
        let Some(code) = code else {
            return Ok(None);
        };
        let Some(row) = self
            .repo()
            .find_by_code(code, scope.shop_id())
            .await?
        else {
            return Ok(None);
        };
        Ok(ConfigDataDTO::from_json_str(row.data.as_deref()))
    }

    /// `POST /adminapi/im/config/config/save`：保存 IM 配置。
    ///
    /// 对应 Java `ImConfigServiceImpl#create(ConfigCreateDTO createDTO)`：
    /// - `code` 必填（`@NotBlank`），空/空白时由 `GlobalExceptionHandler`
    ///   以 `SERVICE_DATA_ERROR` 包装校验文案（见 [`ERR_CODE_NOT_BLANK`]）；
    /// - 按 `code` + `shop_id` 查已有行：存在则**只覆盖 `data`**；
    ///   不存在则插入新行（`code` / `shop_id` / `data`）；
    /// - `data` 为 `null` 时，Java `JsonUtil.toJson(null)` 返回 `null`，
    ///   故既有行会被置为 `NULL`、新行也落 `NULL`——此处保持一致。
    ///
    /// 原接口返回 `void`；本方法返回落库后的主键（新行为新自增 ID，
    /// 行为既有行 ID），供路由层回传便于排查。
    ///
    /// # Errors
    /// - `code` 为空白 → `Error::Validation("must not be blank")`
    ///   （Java `@NotBlank` 经全局异常处理器包装，业务码 400）；
    /// - 数据库失败 → `Error::Database`。
    pub async fn save(&self, dto: &ConfigCreateDTO, scope: ImConfigScope) -> Result<i32> {
        if !is_valid_code(&dto.code) {
            return Err(Error::Validation(ERR_CODE_NOT_BLANK.into()));
        }

        // Java 控制器：`createDTO.setShopId(HeaderUtils.getShopId())`
        // → 请求体里的 shopId 被请求头覆盖；Unscoped 时保持 null
        let shop_id = scope.shop_id();
        let data = dto
            .data
            .as_ref()
            .map(ConfigDataDTO::to_json_string)
            .transpose()?;

        match resolve_save_action(
            self.repo()
                .find_by_code(&dto.code, shop_id)
                .await?
                .map(|row| row.id),
        ) {
            SaveAction::Insert => {
                // Java：`BeanUtils.copyProperties(createDTO, createConfig)` +
                // `setData(...)` + `save`，写入 `code` / `shop_id` / `data`。
                // 注意 `shop_id` 在 `im_config` 中是 NOT NULL：Java 在
                // `HeaderUtils.getShopId()` 为 null（未知账号类型）时会把
                // null 写进去并触发约束异常；此处按 0 落库以避免无意义的 500，
                // 差异见 uncertain_points。
                self.repo()
                    .insert_config(&dto.code, data.as_deref(), shop_id.unwrap_or(0))
                    .await
            }
            SaveAction::Update { existing_id } => {
                self.repo()
                    .update_config_data(existing_id, data.as_deref())
                    .await?;
                Ok(existing_id)
            }
        }
    }
}

// ————————————————————— 纯逻辑辅助（可单测） —————————————————————
//
// 说明：本切片的服务层全部逻辑都建立在一行 `im_config` 的读写之上，
// 没有可独立测试的金额/状态机逻辑。以下辅助函数抽出「查找结果 → 动作」
// 与「data 文本 → DTO」两个纯转换，用单测锁定与 Java 一致的分支语义；
// 真实的 SQL 路径由集成测试覆盖。

/// 保存动作（Java `create` 的 if/else 分支结果）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaveAction {
    /// 未命中已有行 → `INSERT`
    Insert,
    /// 命中已有行（携带其主键）→ `UPDATE ... SET data = ? WHERE id = ?`
    Update { existing_id: i32 },
}

/// 由「按 `code` + `shop_id` 查到的主键」推导保存动作。
///
/// 与 Java `if (config == null) { save } else { updateById }` 一一对应。
#[must_use]
pub fn resolve_save_action(existing_id: Option<i32>) -> SaveAction {
    match existing_id {
        None => SaveAction::Insert,
        Some(id) => SaveAction::Update { existing_id: id },
    }
}

/// 校验 `code` 是否满足 Java `@NotBlank`（非 `null` 且去除首尾空白后非空）。
#[must_use]
pub fn is_valid_code(code: &str) -> bool {
    !code.trim().is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blank_code_is_rejected_like_not_blank() {
        // Java @NotBlank：null / "" / "   " 均不通过
        assert!(!is_valid_code(""));
        assert!(!is_valid_code("   "));
        assert!(!is_valid_code("\t\n"));
        assert!(is_valid_code("kefuConfig"));
        // 首尾空白但有内容 → Java @NotBlank 同样通过（Hibernate 校验器 trim 后判空）
        assert!(is_valid_code(" kefu "));
    }

    #[test]
    fn save_action_follows_java_if_else_branch() {
        assert_eq!(resolve_save_action(None), SaveAction::Insert);
        assert_eq!(
            resolve_save_action(Some(42)),
            SaveAction::Update { existing_id: 42 }
        );
    }

    #[test]
    fn scope_shop_id_filter_semantics() {
        // 平台账号：HeaderUtils.getShopId() == 0 → 过滤 shop_id = 0
        assert_eq!(ImConfigScope::Shop(0).shop_id(), Some(0));
        // 店铺账号：过滤自己的 shop_id
        assert_eq!(ImConfigScope::Shop(9).shop_id(), Some(9));
        // 未知账号类型（Java 返回 null）→ 不加过滤条件
        assert_eq!(ImConfigScope::Unscoped.shop_id(), None);
    }

    #[test]
    fn data_json_round_trip_matches_java_field_names() {
        // 落库文本 ↔ 返回体应能无损往返（等价于 Java JsonUtil.toJson/fromJson）
        let dto = ConfigDataDTO {
            activate: true,
            send_text: false,
            send_wechat: true,
            wechat_image: None,
            reply_content: Some("自动回复".into()),
        };
        let text = dto.to_json_string().unwrap();
        assert!(text.contains("\"sendWechat\":true"));
        assert!(text.contains("\"wechatImage\":null"));
        assert!(!text.contains("send_wechat"));
        let back = ConfigDataDTO::from_json_str(Some(&text)).unwrap();
        assert_eq!(back.activate, dto.activate);
        assert_eq!(back.send_text, dto.send_text);
        assert_eq!(back.send_wechat, dto.send_wechat);
        assert_eq!(back.wechat_image, dto.wechat_image);
        assert_eq!(back.reply_content, dto.reply_content);
    }

    #[test]
    fn detail_returns_none_for_missing_or_broken_data() {
        // Java：data 为 null / 空串 / 非法 JSON / 字面量 "null"
        // → JsonUtil.fromJson 返回 null（Jackson 对 "null" 也返回 null）
        assert_eq!(ConfigDataDTO::from_json_str(None), None);
        assert_eq!(ConfigDataDTO::from_json_str(Some("")), None);
        assert_eq!(ConfigDataDTO::from_json_str(Some("null")), None);
        assert!(ConfigDataDTO::from_json_str(Some(r#"{"activate":true}"#))
            .is_some_and(|d| d.activate));
    }
}