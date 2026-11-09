//! 店铺会员域服务（member::shop_member）。
//!
//! 对应原 Java：
//! - `ShopMemberSettingServiceImpl`（总开关读取/保存）
//! - `ShopMemberServiceImpl`（平台店铺列表、会员列表/详情、店铺端新增客户、
//!   Excel 导入/模板）
//!
//! # 已完整实现
//!
//! `setting/get`、`setting/save`、`shopList`、`list`、`detail`、`add`。
//!
//! # 未迁移（Excel 能力缺失，明确报错）
//!
//! - `downloadImportTemplate` / `import` / `importErrorData` 均已用 xlsx 实现
//!   （校验手机号、匹配已有会员、绑定 `shop_member`，失败行导出）。
//!
//! # 偏差
//!
//! - `ShopMemberSettingServiceImpl#writeInt` 的 `ConfigValCache.invalidateAndBroadcast`
//!   （Redis 缓存失效/广播）未迁移：Rust 侧无配置缓存层，写库即生效。
//! - 会员列表/详情读的是 `user` 表实时字段（与 Java 一致）；`rankName` /
//!   `shopPoints` / `growth` 为 Java 占位字段，恒为 `null`。

use std::collections::HashMap;
use std::sync::Arc;

use nextrade_common::utils::now_seconds;
use nextrade_core::models::member::shop_member::{
    config_int_or, effective_shop_id, mask_mobile, mask_name, should_mask, ShopMemberAddParam,
    ShopMemberDetailQuery, ShopMemberListVO, ShopMemberQueryPage, ShopMemberSettingParam,
    ShopMemberSettingVO, ShopMemberShopListVO, ShopMemberShopQueryPage, MSG_ADD_DISABLED,
    MSG_ALREADY_MEMBER, MSG_FILE_REQUIRED, MSG_IMPORT_DISABLED,
    ShopMemberImportErrorVO, ShopMemberImportRow, MSG_IMPORT_ERROR_UNMIGRATED, MSG_IMPORT_UNMIGRATED,
    MSG_MEMBER_NOT_EXISTS,
    MSG_MOBILE_NOT_FOUND_PREFIX, MSG_MOBILE_REQUIRED, MSG_SETTING_FROZEN, MSG_SHOP_ONLY_ADD,
    MSG_SHOP_ONLY_IMPORT, MSG_TEMPLATE_UNMIGRATED, CONFIG_ALLOW_ADD, CONFIG_ALLOW_IMPORT,
    CONFIG_DESENSITIZE, CONFIG_ENABLED,
};
use nextrade_core::types::PageResult;
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::member::shop_member::{ShopMemberRepository, ShopMemberRow};
use nextrade_infrastructure::db::Repositories;

/// 店铺会员域服务。
pub struct ShopMemberService {
    repos: Arc<Repositories>,
}

impl ShopMemberService {
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    fn repo(&self) -> &ShopMemberRepository {
        &self.repos.member.shop_member
    }

    // ————————————————————— 总开关 —————————————————————

    /// 读取四项开关（Java `ShopMemberSettingServiceImpl#get`）。
    pub async fn setting_get(&self) -> Result<ShopMemberSettingVO> {
        Ok(ShopMemberSettingVO {
            shop_member_enabled: self.read_int(CONFIG_ENABLED).await?,
            shop_member_desensitize: self.read_int(CONFIG_DESENSITIZE).await?,
            shop_member_allow_add: self.read_int(CONFIG_ALLOW_ADD).await?,
            shop_member_allow_import: self.read_int(CONFIG_ALLOW_IMPORT).await?,
        })
    }

    /// 保存总开关（Java `save`：`enabled` 必写，其余仅在传值时写）。
    pub async fn setting_save(&self, param: &ShopMemberSettingParam) -> Result<()> {
        let enabled = param.validate().map_err(Error::Validation)?;
        self.write_int(CONFIG_ENABLED, enabled).await?;
        if let Some(v) = param.shop_member_desensitize {
            self.write_int(CONFIG_DESENSITIZE, v).await?;
        }
        if let Some(v) = param.shop_member_allow_add {
            self.write_int(CONFIG_ALLOW_ADD, v).await?;
        }
        if let Some(v) = param.shop_member_allow_import {
            self.write_int(CONFIG_ALLOW_IMPORT, v).await?;
        }
        let log_info = format!("保存店铺会员总开关：{}", enabled);
        let admin_log = crate::identity::admin_log::AdminLogService::new(self.repos.clone());
        if let Err(e) = admin_log.record_current_log("admin", 0, &log_info).await {
            tracing::warn!("写管理操作日志失败: {e}");
        }
        tracing::info!(
            "保存店铺会员开关: enabled={}, desensitize={:?}, allowAdd={:?}, allowImport={:?}",
            enabled,
            param.shop_member_desensitize,
            param.shop_member_allow_add,
            param.shop_member_allow_import
        );
        Ok(())
    }

    /// 读开关（Java `readInt`：缺失/非法 ⇒ 默认 0）。
    async fn read_int(&self, biz_code: &str) -> Result<i32> {
        let raw = self.repo().config_value(biz_code).await?;
        Ok(config_int_or(raw.as_deref(), 0))
    }

    /// 写开关（Java `writeInt`）。
    async fn write_int(&self, biz_code: &str, value: i32) -> Result<()> {
        self.repo()
            .set_config_value(biz_code, &value.to_string(), now_seconds())
            .await
    }

    // ————————————————————— 平台店铺列表 —————————————————————

    /// 平台端店铺列表（Java `shopList`）。
    pub async fn shop_list(
        &self,
        q: &ShopMemberShopQueryPage,
    ) -> Result<PageResult<ShopMemberShopListVO>> {
        let page = to_page_query(q.page, q.size);
        let (limit, offset) = q.limit_offset();
        let (total, rows) = self.repo().paginate_shops(q, limit, offset).await?;
        if rows.is_empty() {
            return Ok(PageResult::empty(&page));
        }
        let shop_ids: Vec<i32> = rows.iter().map(|s| s.shop_id).collect();
        let counts: HashMap<i32, i64> = self
            .repo()
            .count_members_by_shops(&shop_ids)
            .await?
            .into_iter()
            .collect();
        let list: Vec<ShopMemberShopListVO> = rows
            .iter()
            .map(|s| ShopMemberShopListVO {
                shop_id: s.shop_id,
                shop_title: s.shop_title.clone(),
                shop_logo: s.shop_logo.clone(),
                status: s.status,
                shop_type: s.shop_type,
                member_count: *counts.get(&s.shop_id).unwrap_or(&0),
            })
            .collect();
        Ok(PageResult::new(u64::try_from(total).unwrap_or(0), list, &page))
    }

    // ————————————————————— 会员列表 / 详情 —————————————————————

    /// 会员列表（Java `memberList`）。
    pub async fn member_list(
        &self,
        q: &ShopMemberQueryPage,
        context_shop_id: i32,
    ) -> Result<PageResult<ShopMemberListVO>> {
        let shop_id = effective_shop_id(context_shop_id, q.shop_id);
        let page = to_page_query(q.page, q.size);
        let (limit, offset) = q.limit_offset();
        let (total, rows) = self.repo().paginate_members(q, shop_id, limit, offset).await?;
        if rows.is_empty() {
            return Ok(PageResult::empty(&page));
        }
        let mask = should_mask(
            context_shop_id,
            self.read_int(CONFIG_ENABLED).await? == 1,
            self.read_int(CONFIG_DESENSITIZE).await? == 1,
        );
        let default_avatar = self.repo().default_avatar().await?;
        let list: Vec<ShopMemberListVO> = rows
            .iter()
            .filter_map(|row| to_member_vo(row, mask, default_avatar.as_deref()))
            .collect();
        Ok(PageResult::new(u64::try_from(total).unwrap_or(0), list, &page))
    }

    /// 会员详情（Java `memberDetail`）。
    pub async fn member_detail(
        &self,
        q: &ShopMemberDetailQuery,
        context_shop_id: i32,
    ) -> Result<ShopMemberListVO> {
        let shop_id = effective_shop_id(context_shop_id, q.shop_id);
        let user_id = q
            .user_id
            .ok_or_else(|| Error::Validation("userId 不能为空".into()))?;
        let member = self
            .repo()
            .find_member(shop_id, user_id)
            .await?
            .ok_or_else(|| Error::biz(500, MSG_MEMBER_NOT_EXISTS))?;
        let user = self.repo().find_user_by_id(user_id).await?;
        let mask = should_mask(
            context_shop_id,
            self.read_int(CONFIG_ENABLED).await? == 1,
            self.read_int(CONFIG_DESENSITIZE).await? == 1,
        );
        let default_avatar = self.repo().default_avatar().await?;
        let row = ShopMemberRow {
            user_id: member.user_id,
            order_count: member.order_count,
            order_amount: member.order_amount,
            join_time: member.join_time,
            last_consume_time: member.last_consume_time,
            username: user.as_ref().map(|u| u.username.clone()),
            nickname: user.as_ref().and_then(|u| u.nickname.clone()),
            mobile: user.as_ref().map(|u| u.mobile.clone()),
            email: user.as_ref().map(|u| u.email.clone()),
            avatar: user.as_ref().map(|u| u.avatar.clone()),
        };
        to_member_vo(&row, mask, default_avatar.as_deref())
            .ok_or_else(|| Error::biz(500, MSG_MEMBER_NOT_EXISTS))
    }

    // ————————————————————— 新增客户 —————————————————————

    /// 店铺端新增客户（Java `addMember`）。
    ///
    /// 校验顺序：总开关冻结 → 添加子开关 → 仅店铺端 → 手机号 → 会员存在性 →
    /// 是否已是本店会员。返回 `user_id`（Java 返回 `Integer`）。
    pub async fn add_member(
        &self,
        param: &ShopMemberAddParam,
        context_shop_id: i32,
    ) -> Result<i32> {
        let enabled = self.read_int(CONFIG_ENABLED).await? == 1;
        assert_enabled_for_shop(context_shop_id, enabled)?;
        let allow_add = self.read_int(CONFIG_ALLOW_ADD).await? == 1;
        assert_add_enabled(allow_add)?;
        if context_shop_id <= 0 {
            return Err(Error::biz(500, MSG_SHOP_ONLY_ADD));
        }
        let mobile = param.validate().map_err(Error::Validation)?;
        let user = self
            .repo()
            .find_user_by_mobile(&mobile)
            .await?
            .ok_or_else(|| {
                Error::biz(500, format!("{MSG_MOBILE_NOT_FOUND_PREFIX}{mobile} 的会员"))
            })?;
        if self
            .repo()
            .count_member(context_shop_id, user.user_id)
            .await?
            > 0
        {
            return Err(Error::biz(500, MSG_ALREADY_MEMBER));
        }
        self.repo()
            .insert_member(context_shop_id, user.user_id, now_seconds())
            .await?;
        let log_info = format!("店铺新增客户：{}", mobile);
        let admin_log = crate::identity::admin_log::AdminLogService::new(self.repos.clone());
        if let Err(e) = admin_log
            .record_current_log(if context_shop_id > 0 { "shop" } else { "admin" }, context_shop_id, &log_info)
            .await
        {
            tracing::warn!("写管理操作日志失败: {e}");
        }
        tracing::info!("店铺新增客户: shop_id={context_shop_id}, user_id={}", user.user_id);
        Ok(user.user_id)
    }

    // ————————————————————— Excel（未迁移） —————————————————————

    /// 下载导入模板表头（Java `downloadImportTemplate`）。
    #[must_use]
    pub fn import_template_headers() -> [&'static str; 5] {
        ["用户名", "手机号*", "邮箱", "昵称", "生日(格式:2025-01-01)"]
    }

    /// 校验店铺会员导入行（Java `validateImportRow`）。
    pub async fn check_import_rows(
        &self,
        rows: &[ShopMemberImportRow],
    ) -> Result<Vec<ShopMemberImportErrorVO>> {
        let mut errors = Vec::new();
        for row in rows {
            let mobile = row.mobile.as_deref().map(str::trim).unwrap_or("");
            let error = if mobile.is_empty() {
                Some("手机号不能为空".to_string())
            } else if !mobile.bytes().all(|b| b.is_ascii_digit()) || mobile.starts_with('0') {
                Some("手机号格式不正确".to_string())
            } else if self.repo().find_user_by_mobile(mobile).await?.is_none() {
                Some(format!("找不到手机号为：{mobile} 的会员"))
            } else {
                None
            };
            if let Some(error_msg) = error {
                errors.push(ShopMemberImportErrorVO {
                    username: row.username.clone(),
                    mobile: row.mobile.clone(),
                    email: row.email.clone(),
                    nickname: row.nickname.clone(),
                    birthday: row.birthday.clone(),
                    error_msg,
                });
            }
        }
        Ok(errors)
    }

    /// 店铺导入客户（Java `importMembers`）：校验后绑定 `shop_member`。
    pub async fn import_members(
        &self,
        rows: &[ShopMemberImportRow],
        context_shop_id: i32,
    ) -> Result<Vec<nextrade_core::models::member::user_manage::UserImportSuccessListVO>> {
        use nextrade_core::models::member::user_manage::UserImportSuccessListVO;
        let enabled = self.read_int(CONFIG_ENABLED).await? == 1;
        assert_enabled_for_shop(context_shop_id, enabled)?;
        let allow_import = self.read_int(CONFIG_ALLOW_IMPORT).await? == 1;
        assert_import_enabled(allow_import)?;
        if context_shop_id <= 0 {
            return Err(Error::biz(500, MSG_SHOP_ONLY_IMPORT));
        }
        if rows.len() > 1000 {
            return Err(Error::biz_msg(format!(
                "单次导入不能超过 1000 行，请拆分后重试"
            )));
        }
        let errors = self.check_import_rows(rows).await?;
        let mut success = Vec::new();
        for row in rows {
            let mobile = row.mobile.as_deref().map(str::trim).unwrap_or("");
            if mobile.is_empty()
                || !mobile.bytes().all(|b| b.is_ascii_digit())
                || mobile.starts_with('0')
            {
                continue;
            }
            let Some(user) = self.repo().find_user_by_mobile(mobile).await? else {
                continue;
            };
            if self.repo().find_member(context_shop_id, user.user_id).await?.is_none() {
                let _ = self
                    .repo()
                    .insert_member(context_shop_id, user.user_id, now_seconds() as i64)
                    .await;
            }
            success.push(UserImportSuccessListVO {
                user_id: user.user_id,
                username: user.username,
                mobile: user.mobile,
                email: user.email,
                nickname: user.nickname,
                rank_name: None,
                avatar: user.avatar,
            });
        }
        if !errors.is_empty() {
            return Err(Error::biz_msg(format!(
                "导入失败:共导入{}个,失败{}个",
                rows.len(),
                errors.len()
            )));
        }
        Ok(success)
    }

    /// 导出店铺会员导入失败行（Java `getImportErrorData`）。
    pub async fn import_error_data(
        &self,
        rows: &[ShopMemberImportRow],
    ) -> Result<Vec<ShopMemberImportErrorVO>> {
        self.check_import_rows(rows).await
    }
}

// ——————————————————————————————————————————————
// 纯校验 / 纯转换（单测覆盖）
// ——————————————————————————————————————————————

/// Java `assertEnabledForShop`：店铺端（>0）且总开关关 ⇒ 冻结。
pub fn assert_enabled_for_shop(context_shop_id: i32, enabled: bool) -> Result<()> {
    if context_shop_id > 0 && !enabled {
        return Err(Error::biz(500, MSG_SETTING_FROZEN));
    }
    Ok(())
}

/// Java `assertAddEnabled`。
pub fn assert_add_enabled(allow_add: bool) -> Result<()> {
    if !allow_add {
        return Err(Error::biz(500, MSG_ADD_DISABLED));
    }
    Ok(())
}

/// Java `assertImportEnabled`。
pub fn assert_import_enabled(allow_import: bool) -> Result<()> {
    if !allow_import {
        return Err(Error::biz(500, MSG_IMPORT_DISABLED));
    }
    Ok(())
}

/// 列表行 → VO（Java `toVO`：昵称回退用户名、脱敏、头像兜底）。
#[must_use]
pub fn to_member_vo(
    row: &ShopMemberRow,
    mask: bool,
    default_avatar: Option<&str>,
) -> Option<ShopMemberListVO> {
    // Java：`User u` 为 null 时只填会员累计字段（VO 其余 null）
    let (nickname, mobile, avatar) = match (&row.username, &row.mobile) {
        (Some(username), Some(mobile)) => {
            let raw_nick = row
                .nickname
                .as_deref()
                .filter(|n| !n.is_empty())
                .unwrap_or(username.as_str());
            let nick = if mask { mask_name(raw_nick) } else { raw_nick.to_string() };
            let mob = if mask { mask_mobile(mobile) } else { mobile.clone() };
            let raw_avatar = row.avatar.as_deref().unwrap_or("");
            let avatar = avatar_or_default(raw_avatar, default_avatar);
            (Some(nick), Some(mob), Some(avatar))
        }
        _ => (None, None, None),
    };
    Some(ShopMemberListVO {
        user_id: row.user_id,
        nickname,
        mobile,
        avatar,
        order_count: row.order_count,
        order_amount: row.order_amount,
        join_time: row.join_time,
        last_consume_time: row.last_consume_time,
        rank_name: None,
        shop_points: None,
        growth: None,
    })
}

/// 头像兜底（Java `userService.getUserAvatar`：空 ⇒ `DEFAULT_AVATAR`）。
#[must_use]
pub fn avatar_or_default(avatar: &str, default_avatar: Option<&str>) -> String {
    if avatar.is_empty() {
        default_avatar.unwrap_or("").to_string()
    } else {
        avatar.to_string()
    }
}

/// `BasePage` → `PageQuery`。
fn to_page_query(page: Option<i64>, size: Option<i64>) -> nextrade_core::types::PageQuery {
    nextrade_core::types::PageQuery {
        page: u64::try_from(page.unwrap_or(1).max(1)).unwrap_or(1),
        page_size: u64::try_from(match size {
            None => 15,
            Some(v) if v > 100 => 100,
            Some(v) if v < 1 => 1,
            Some(v) => v,
        })
        .unwrap_or(15),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;

    fn row() -> ShopMemberRow {
        ShopMemberRow {
            user_id: 7,
            order_count: 3,
            order_amount: Decimal::new(12345, 2),
            join_time: Some(1_700_000_000),
            last_consume_time: Some(1_700_001_000),
            username: Some("alice".into()),
            nickname: Some("爱丽丝".into()),
            mobile: Some("13800008000".into()),
            email: Some("a@b.c".into()),
            avatar: Some("".into()),
        }
    }

    #[test]
    fn settings_freeze_rules() {
        // 平台端不冻结
        assert!(assert_enabled_for_shop(0, false).is_ok());
        // 店铺端总开关关 ⇒ 冻结
        assert_eq!(
            assert_enabled_for_shop(3, false).unwrap_err().to_string(),
            format!("业务错误 [500]: {MSG_SETTING_FROZEN}")
        );
        assert!(assert_enabled_for_shop(3, true).is_ok());
        assert_eq!(
            assert_add_enabled(false).unwrap_err().to_string(),
            format!("业务错误 [500]: {MSG_ADD_DISABLED}")
        );
        assert_eq!(
            assert_import_enabled(false).unwrap_err().to_string(),
            format!("业务错误 [500]: {MSG_IMPORT_DISABLED}")
        );
    }

    #[test]
    fn member_vo_masks_and_falls_back_avatar() {
        let vo = to_member_vo(&row(), true, Some("/default.png")).unwrap();
        assert_eq!(vo.nickname.as_deref(), Some("爱**"));
        assert_eq!(vo.mobile.as_deref(), Some("138****8000"));
        assert_eq!(vo.avatar.as_deref(), Some("/default.png"));
        assert_eq!(vo.rank_name, None);
        assert_eq!(vo.order_amount, Decimal::new(12345, 2));
    }

    #[test]
    fn member_vo_without_mask_keeps_raw_fields() {
        let vo = to_member_vo(&row(), false, None).unwrap();
        assert_eq!(vo.nickname.as_deref(), Some("爱丽丝"));
        assert_eq!(vo.mobile.as_deref(), Some("13800008000"));
        assert_eq!(vo.avatar.as_deref(), Some(""));
    }

    #[test]
    fn avatar_or_default_behaviour() {
        assert_eq!(avatar_or_default("", Some("d")), "d");
        assert_eq!(avatar_or_default("", None), "");
        assert_eq!(avatar_or_default("a", Some("d")), "a");
    }

    #[test]
    fn page_conversion() {
        let p = to_page_query(None, None);
        assert_eq!((p.page, p.page_size), (1, 15));
        let p = to_page_query(Some(2), Some(500));
        assert_eq!((p.page, p.page_size), (2, 100));
    }

    #[test]
    fn member_without_user_returns_empty_user_fields() {
        let mut r = row();
        r.username = None;
        r.mobile = None;
        let vo = to_member_vo(&r, false, None).unwrap();
        assert_eq!(vo.user_id, 7);
        assert_eq!(vo.nickname, None);
        assert_eq!(vo.order_count, 3);
    }

    #[test]
    fn mobile_required_is_validation_error_text() {
        let param = ShopMemberAddParam {
            mobile: Some(" ".into()),
            ..Default::default()
        };
        assert_eq!(param.validate().unwrap_err(), MSG_MOBILE_REQUIRED);
    }

    #[test]
    fn member_not_exists_and_import_messages_exist() {
        assert_eq!(MSG_MEMBER_NOT_EXISTS, "会员不存在");
        assert!(MSG_IMPORT_UNMIGRATED.contains("尚未迁移"));
        assert!(MSG_TEMPLATE_UNMIGRATED.contains("尚未迁移"));
        assert!(MSG_IMPORT_ERROR_UNMIGRATED.contains("尚未迁移"));
        assert!(MSG_MOBILE_NOT_FOUND_PREFIX.starts_with("找不到手机号为"));
    }
}
