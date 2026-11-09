//! 邮箱模板管理域服务（system::mail_templates）。
//!
//! 对应原 Java `com.nextrade.service.setting.impl.MailTemplatesServiceImpl`
//! （`MailTemplatesController` 的 2 个接口）。
//!
//! 构造签名固定 `pub fn new(repos: Arc<Repositories>) -> Self`。
//!
//! ## 逐条对照 Java
//!
//! - `mailTemplatesList()` → [`MailTemplatesService::list`]：
//!   `this.list()` 全表查询 → 逐行 `BeanUtils.copyProperties` →
//!   `vo.setTemplateName(MailTemplateType.fromCode(templateCode))`。
//! - `updateMailTemplates(dto)` → [`MailTemplatesService::update`]：
//!   校验 → 构造补丁（`is_html` / `template_subject` / `template_content`
//!   / `last_modify`）→ `updateById`。
//!
//! ## 审计日志
//!
//! 更新成功后写 `admin_log`（`AdminLogService::record_log`），actor 取 JWT
//! `AuthUser.id`、IP 取请求头 `X-Forwarded-For`/`X-Real-IP`。

use std::sync::Arc;

use nextrade_common::utils;
use nextrade_core::models::system::mail_templates::{
    MailTemplatesDTO, MailTemplatesVO, MSG_TEMPLATE_CONTENT_NOT_NULL, MSG_TEMPLATE_ID_NOT_NULL,
    MSG_TEMPLATE_SUBJECT_MAX_LENGTH, MSG_TEMPLATE_SUBJECT_NOT_NULL, TEMPLATE_SUBJECT_MAX_LEN,
};
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::system::mail_templates::{MailTemplatesPatch, MailTemplatesRepository};
use nextrade_infrastructure::db::Repositories;

/// 邮箱模板管理 域服务。
pub struct MailTemplatesService {
    repos: Arc<Repositories>,
}

impl MailTemplatesService {
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    /// 本片仓储。
    #[must_use]
    pub fn repo(&self) -> &MailTemplatesRepository {
        &self.repos.system.mail_templates
    }

    /// 邮箱模板列表。
    ///
    /// 对应 Java `MailTemplatesServiceImpl#mailTemplatesList`
    /// （`MailTemplatesController.list`，`GET /adminapi/setting/mailTemplates/list`）。
    ///
    /// 返回值顺序 = 仓储 `ORDER BY template_id ASC`（见仓储说明）。
    pub async fn list(&self) -> Result<Vec<MailTemplatesVO>> {
        let rows = self.repo().find_all().await?;
        Ok(rows.iter().map(MailTemplatesVO::from).collect())
    }

    /// 更新邮件模板。
    ///
    /// 对应 Java `MailTemplatesServiceImpl#updateMailTemplates`
    /// （`MailTemplatesController.update`，`POST /adminapi/setting/mailTemplates/update`）。
    ///
    /// 校验（逐条复刻 Java DTO 注解）：
    /// 1. `templateId` `@NotNull` → [`MSG_TEMPLATE_ID_NOT_NULL`]
    /// 2. `templateSubject` `@NotBlank` → [`MSG_TEMPLATE_SUBJECT_NOT_NULL`]
    /// 3. `templateSubject` `@Size(max = 100)` → [`MSG_TEMPLATE_SUBJECT_MAX_LENGTH`]
    /// 4. `templateContent` `@NotBlank` → [`MSG_TEMPLATE_CONTENT_NOT_NULL`]
    ///
    /// ⚠️ Java 的 `updateById` 命中 0 行（主键不存在）时**不报错**，
    /// 本实现一致：静默返回 `Ok(())`（仅审计日志会少一条）。
    pub async fn update(
        &self,
        dto: &MailTemplatesDTO,
        admin_id: i32,
        client_ip: &str,
    ) -> Result<()> {
        let template_id = validate_update(dto)?;
        let patch = build_patch(dto, utils::now_seconds());

        let affected = self.repo().update(template_id, &patch).await?;
        if affected == 0 {
            // 与 Java 一致：主键不存在时 updateById 返回 false 且不抛异常。
            tracing::warn!("更新邮件模板未命中任何行: template_id={template_id}");
        }
        // Java：adminLogService.createByLogInfo(StrUtil.format("更新邮件模板设置:{}", 主题))
        let log_info = format!("更新邮件模板设置:{}", patch.template_subject);
        let admin_log = crate::identity::admin_log::AdminLogService::new(self.repos.clone());
        if let Err(e) = admin_log
            .record_log(admin_id, "admin", 0, &log_info, Some(client_ip), None)
            .await
        {
            tracing::warn!("写管理操作日志失败（更新邮件模板）: {e}");
        }
        tracing::info!(
            "更新邮件模板设置: template_id={template_id}, template_subject={}",
            patch.template_subject
        );
        Ok(())
    }
}

/// 复刻 Java `java.lang.String#trim()` 的「空白」判定：
/// Java `@NotBlank` 等价于 `!value.trim().isEmpty()`，
/// 而 `String.trim()` **只**去掉 `<= U+0020` 的字符
/// （不包含 `U+00A0` 等 Unicode 空白）。
///
/// `trim()` 后非空 ⇔ 存在一个 `> U+0020` 的字符，故本函数如此实现
/// —— 比 Rust `str::trim()`（按 Unicode 空白）更贴近 Java 语义。
fn is_java_blank(s: &str) -> bool {
    !s.chars().any(|c| c > '\u{20}')
}

/// 校验 `MailTemplatesDTO` 并返回 `template_id`（纯函数，便于单测）。
///
/// 校验顺序与 Java DTO 的字段声明顺序一致：
/// `templateId` → `templateSubject`（非空 → 长度）→ `templateContent`。
fn validate_update(dto: &MailTemplatesDTO) -> Result<i32> {
    let template_id = dto
        .template_id
        .ok_or_else(|| Error::Validation(MSG_TEMPLATE_ID_NOT_NULL.to_string()))?;

    let subject = dto.template_subject.as_deref().unwrap_or("");
    if is_java_blank(subject) {
        return Err(Error::Validation(MSG_TEMPLATE_SUBJECT_NOT_NULL.to_string()));
    }
    // Java `@Size(max = 100)` 以 `String.length()`（UTF-16 code unit）计长，
    // 故用 `encode_utf16().count()` 而非 `chars().count()`。
    if subject.encode_utf16().count() > TEMPLATE_SUBJECT_MAX_LEN {
        return Err(Error::Validation(
            MSG_TEMPLATE_SUBJECT_MAX_LENGTH.to_string(),
        ));
    }

    if is_java_blank(dto.template_content.as_deref().unwrap_or("")) {
        return Err(Error::Validation(MSG_TEMPLATE_CONTENT_NOT_NULL.to_string()));
    }

    Ok(template_id)
}

/// 由 DTO 构造仓储更新补丁（纯函数，便于单测）。
///
/// - `is_html`：DTO 的 `isHtml`（`None` ⇒ MyBatis-Plus `NOT_NULL` 语义下不写该列）
/// - `template_subject` / `template_content`：已由 [`validate_update`] 保证非空
/// - `last_modify`：`now` 截断为 `i32`（Java `StringUtils.getCurrentTime()` 秒级时间戳，
///   库列为 `int unsigned`；`u32::MAX < i32::MAX` 之外才会截断，实际不会发生）
fn build_patch(dto: &MailTemplatesDTO, now: i64) -> MailTemplatesPatch {
    MailTemplatesPatch {
        is_html: dto.is_html,
        template_subject: dto.template_subject.clone().unwrap_or_default(),
        template_content: dto.template_content.clone().unwrap_or_default(),
        last_modify: i32::try_from(now).unwrap_or(i32::MAX),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dto(id: Option<i32>, is_html: Option<i32>, subject: &str, content: &str) -> MailTemplatesDTO {
        MailTemplatesDTO {
            template_id: id,
            is_html,
            template_subject: Some(subject.to_string()),
            template_content: Some(content.to_string()),
        }
    }

    #[test]
    fn validate_rejects_missing_template_id() {
        let d = dto(None, Some(1), "主题", "正文");
        let err = validate_update(&d).unwrap_err();
        assert_eq!(err.to_string(), format!("参数校验失败: {MSG_TEMPLATE_ID_NOT_NULL}"));
    }

    #[test]
    fn validate_rejects_blank_subject_like_not_blank() {
        // 空串 / 纯 ASCII 空白 / 制表换行都算 blank
        for s in ["", "   ", "\t\n"] {
            let d = dto(Some(1), None, s, "正文");
            assert_eq!(
                validate_update(&d).unwrap_err().to_string(),
                format!("参数校验失败: {MSG_TEMPLATE_SUBJECT_NOT_NULL}"),
                "subject={s:?} 应判为空白"
            );
        }
        // Java String.trim() 不去 U+00A0 ⇒ 非 blank
        let nb = dto(Some(1), None, "\u{00a0}", "正文");
        assert!(validate_update(&nb).is_ok());
    }

    #[test]
    fn validate_subject_length_boundary() {
        // 100 个 ASCII ⇒ 通过；101 ⇒ 报长度错
        let ok = dto(Some(1), None, &"a".repeat(TEMPLATE_SUBJECT_MAX_LEN), "正文");
        assert!(validate_update(&ok).is_ok());

        let too_long = dto(
            Some(1),
            None,
            &"a".repeat(TEMPLATE_SUBJECT_MAX_LEN + 1),
            "正文",
        );
        assert_eq!(
            validate_update(&too_long).unwrap_err().to_string(),
            format!("参数校验失败: {MSG_TEMPLATE_SUBJECT_MAX_LENGTH}")
        );
    }

    #[test]
    fn validate_subject_length_counts_utf16_units() {
        // 每个 emoji 是 2 个 UTF-16 code unit：50 个 = 100 ⇒ 边界通过
        let fifty = "😀".repeat(50);
        assert_eq!(fifty.encode_utf16().count(), 100);
        assert!(validate_update(&dto(Some(1), None, &fifty, "正文")).is_ok());
        // 51 个 = 102 ⇒ 超限（若用 chars().count() 会误判为 51 而通过）
        let fifty_one = "😀".repeat(51);
        assert!(validate_update(&dto(Some(1), None, &fifty_one, "正文")).is_err());
    }

    #[test]
    fn validate_rejects_missing_content_and_missing_subject() {
        let d = MailTemplatesDTO {
            template_id: Some(1),
            is_html: None,
            template_subject: None,
            template_content: Some("正文".into()),
        };
        assert_eq!(
            validate_update(&d).unwrap_err().to_string(),
            format!("参数校验失败: {MSG_TEMPLATE_SUBJECT_NOT_NULL}")
        );

        let d = MailTemplatesDTO {
            template_id: Some(1),
            is_html: None,
            template_subject: Some("主题".into()),
            template_content: None,
        };
        assert_eq!(
            validate_update(&d).unwrap_err().to_string(),
            format!("参数校验失败: {MSG_TEMPLATE_CONTENT_NOT_NULL}")
        );
    }

    #[test]
    fn build_patch_keeps_not_null_semantics_and_last_modify() {
        // isHtml 缺失 ⇒ None（不写该列）
        let d = dto(Some(9), None, "主题", "正文");
        let p = build_patch(&d, 1_700_000_000);
        assert_eq!(p.is_html, None);
        assert_eq!(p.template_subject, "主题");
        assert_eq!(p.template_content, "正文");
        assert_eq!(p.last_modify, 1_700_000_000);

        // isHtml = 0 必须照写（0 不是 null，NOT_NULL 策略下会写）
        let d = dto(Some(9), Some(0), "主题", "正文");
        assert_eq!(build_patch(&d, 0).is_html, Some(0));
    }
}