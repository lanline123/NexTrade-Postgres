//! 买家秀评论管理服务（marketing::buyer_show_comment）：评论分页、审核、批量审核、公开/隐藏、置顶。
//!
//! 对应原 Java `BuyerShowCommentServiceImpl`
//! （`java/nextrade-service/src/main/java/com/nextrade/service/promotion/impl/BuyerShowCommentServiceImpl.java`）
//! 中由 `BuyerShowCommentController`（`@RequestMapping("/adminapi/promotion/buyerShow/commentInfo")`）
//! 暴露的五条链路：
//!
//! | 本模块方法 | 原 Java | 说明 |
//! |-----------|--------|------|
//! | [`BuyerShowCommentService::list`] | `BuyerShowCommentServiceImpl#list` | 分页 + 发布人/状态文案派生 |
//! | [`BuyerShowCommentService::audit`] | `#audit` | 写 `status` + 由 `status` 推导 `is_show` |
//! | [`BuyerShowCommentService::batch`] | `#batch` | 仅 `type = "audit"` 有副作用 |
//! | [`BuyerShowCommentService::show_or_hide`] | `#showOrHide` | 仅写 `is_show` |
//! | [`BuyerShowCommentService::is_top`] | `#isTop` | 仅写 `is_top` |
//!
//! ## 逐条对照 Java 的语义要点
//!
//! 1. **列表过滤**：`status == -1` → `status = 0`（待审核）；`status > -1` →
//!    `status <> 0 AND is_show = status`；`status` 为空或 `< -1` → 不过滤。
//!    时间区间仅在 `startTime` 与 `endTime` **都非空**时生效，且按 `between`
//!    闭区间、两端都取当日 `00:00:00`（Java `TigUtils.toTimestampYmd` 不补 23:59:59）。
//! 2. **发布人文案**：Java 先 `BeanUtils.copyProperties(entity, vo)`（此时 VO 尚无语义），
//!    再 `buyerShowComment.setUserName(user.getName())`（只改**实体**），
//!    最后 `vo.setUserName(entity.getUserName() + type)`。由于 copyProperties 发生在
//!    赋值之前，VO 的 `userName` 实际为 `实体 user_name + "(系统)" | "(客户)"`
//!    （**查到的会员昵称/用户名并未生效**——Java 侧赋值对象写错，本实现**照实复刻**，
//!    以保证与线上返回逐字一致；见 `uncertain_points`）。
//! 3. **状态文案**：`status == 0` → 「待审核」；否则 `is_show == 1` → 「公开」，其余「隐藏」。
//! 4. **审核**：`is_show = (status == 1 ? 1 : 0)`；**不校验** `status` 合法取值（Java 同样原样落库）。
//! 5. **批量**：`type != "audit"` 时 Java `return false` 且无任何副作用，此处一致；
//!    `value` 同时写入 `status` 与推导出的 `is_show`。
//! 6. **单条更新（audit / showOrHide / isTop）**：Java 不检查受影响行数，
//!    更新 0 行也算成功；本实现同样忽略行数（仅记 `tracing` 日志）。
//!
//! ## 未迁移 / 无依赖
//!
//! 本切片不依赖对象存储、支付、Redis、翻译或配置服务；`list` 只多取一张 `user`
//! 表（同 Java 的 `userService.lambdaQuery()`）。
//!
//! ## 与 Java 的已知差异
//!
//! - **权限**：Java 用 `@PreAuthorize("@pms.hasPermission('buyerShowManage' / 'buyerShowUpdateManage'))`
//!   做方法级鉴权；Rust 侧 \"require_auth\" 只做登录态校验，权限矩阵尚未实现
//!   （与 seckill.rs / order_admin.rs 同一已知缺口）。
//! - **参数校验**：Java `@NotNull` / `@NotEmpty` 在进服务前由 Spring 拦截，
//!   本实现把同样文案的校验放在服务层（见 [\"validate_audit_param\"] 等）。
//! - **分页结构**：Java 返回 MyBatis-Plus `Page<BuyerShowCommentPageVO>`
//!   （`records/total/size/current/pages`），Rust 统一用项目既有 \"PageResult\"
//!   （`list/total/totalPage/pageSize/page`），与 order_admin / stock_warning 同一约定。

//! # 多租户（N/A 判定）
//!
//! 本域为**平台级数据**，不做店铺隔离（N/A），证据：
//! - `java/database/pure.sql` 中 \"buyer_show_comment\" 表**没有 \"shop_id\" 列**；
//! - 原 Java \"BuyerShowCommentServiceImpl\" 中**没有任何** `getShopId()` / \"shop_id\" 引用；
//! - 因此管理端列表/详情/写操作均以平台身份执行，接入 \"TenantScope\"
//!   不会改变行为（平台作用域默认放行，店铺账号访问时按平台数据可见性处理）。
//!
//! 若未来产品要求店铺级隔离，需先给 \"buyer_show_comment\" 增加 \"shop_id\" 列并回填归属。
//!
use std::collections::HashMap;
use std::sync::Arc;

use nextrade_core::entities::{BuyerShowComment, User};
use nextrade_core::models::marketing::buyer_show_comment::{
    BatchDTO, BuyerShowCommentAuditParam, BuyerShowCommentIsTopParam, BuyerShowCommentListQuery,
    BuyerShowCommentPageVO, BuyerShowCommentShowOrHideParam,
};
use nextrade_core::types::PageResult;
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::Repositories;

// ——————————————————————————————————————————————————————————————
// 错误文案（逐字对齐 Java / \"ResultTextConstants\"）
// ——————————————————————————————————————————————————————————————

/// Java `BuyerShowCommentAuditParam.buyerShowCommentId` 的 `@NotNull` 文案。
pub const ERR_COMMENT_ID_REQUIRED: &str = "买家秀评论ID不能为空";
/// Java `BuyerShowCommentAuditParam.status` 的 `@NotNull` 文案。
pub const ERR_AUDIT_STATUS_REQUIRED: &str = "审核状态不能为空";
/// Java `BuyerShowCommentShowOrHideParam.isShow` 的 `@NotNull` 文案。
pub const ERR_IS_SHOW_REQUIRED: &str = "审核状态不能为空";
/// Java `BuyerShowCommentIsTopParam.isTop` 的 `@NotNull` 文案。
pub const ERR_IS_TOP_REQUIRED: &str = "是否置顶不能为空";
/// Java `ResultTextConstants.UNSELECTED_ITEM`（`BatchDTO.ids` 的 `@NotEmpty` 文案）。
pub const ERR_UNSELECTED_ITEM: &str = "未选择项目";
/// Java `ResultTextConstants.OPERATION_TYPE_IS_NULL`（`BatchDTO.type` 的 `@NotNull` 文案）。
pub const ERR_OPERATION_TYPE_REQUIRED: &str = "操作类型不能为空";

// ——————————————————————————————————————————————————————————————
// 纯函数（可单测）
// ——————————————————————————————————————————————————————————————

/// 秒级时间戳 → `yyyy-MM-dd HH:mm:ss`。
///
/// 对齐 Java `TigUtils.handelTime(Long)`：`time == null || time == 0` 返回空串。
/// 注意 Java 只在 `== 0` 时短路（负数会被 `new Date(负数)` 格式化成 1969 年的字符串），
/// 这里保持一致。
#[must_use]
pub fn fmt_time(secs: i64) -> String {
    if secs == 0 {
        return String::new();
    }
    use chrono::TimeZone;
    match chrono::Local.timestamp_opt(secs, 0) {
        chrono::LocalResult::Single(dt) => dt.format("%Y-%m-%d %H:%M:%S").to_string(),
        _ => String::new(),
    }
}

/// Java `User#getName()`：`StrUtil.isNotBlank(nickname) ? nickname : username`。
#[must_use]
pub fn user_display_name(user: &User) -> String {
    match user.nickname.as_deref().map(str::trim) {
        Some(nickname) if !nickname.is_empty() => nickname.to_string(),
        _ => user.username.clone(),
    }
}

/// 发布人后缀（Java `comment.getUserId() == 0 ? "(系统)" : "(客户)"`）。
#[must_use]
pub fn user_type_suffix(user_id: i32) -> &'static str {
    if user_id == 0 {
        "(系统)"
    } else {
        "(客户)"
    }
}

/// 列表状态文案。
///
/// 对应 Java：
/// `status == 0 ? "待审核" : isShow == 1 ? "公开" : "隐藏"`。
#[must_use]
pub fn status_text(status: i32, is_show: i32) -> &'static str {
    if status == 0 {
        "待审核"
    } else if is_show == 1 {
        "公开"
    } else {
        "隐藏"
    }
}

/// 发布人展示值。
///
/// 复刻 Java 的实际行为（见模块头注释第 2 条）：实体 `user_name` + 类型后缀；
/// 会员信息（`user_map`）在 Java 中被赋给了实体而非 VO，故顺序上**不参与**结果，
/// 这里仅保留参数以标注该依赖，避免日后误以为取了会员昵称。
#[must_use]
pub fn build_user_name(comment: &BuyerShowComment, user_map: &HashMap<i32, String>) -> String {
    let _ = user_map;
    format!("{}{}", comment.user_name, user_type_suffix(comment.user_id))
}

/// 实体 → 列表 VO（对应 Java \"list\" 循环内的字段构造）。
#[must_use]
pub fn to_page_vo(
    comment: &BuyerShowComment,
    user_map: &HashMap<i32, String>,
) -> BuyerShowCommentPageVO {
    BuyerShowCommentPageVO {
        buyer_show_id: comment.buyer_show_id,
        buyer_show_comment_id: comment.buyer_show_comment_id,
        add_time: fmt_time(comment.add_time),
        user_name: build_user_name(comment, user_map),
        is_top: comment.is_top,
        status: comment.status,
        is_show: comment.is_show,
        status_text: status_text(comment.status, comment.is_show).to_string(),
        content: comment.content.clone(),
    }
}

/// 审核参数校验（Java `@Validated BuyerShowCommentAuditParam` 的两个 `@NotNull`）。
pub fn validate_audit_param(param: &BuyerShowCommentAuditParam) -> Result<()> {
    if param.buyer_show_comment_id.is_none() {
        return Err(Error::Validation(ERR_COMMENT_ID_REQUIRED.into()));
    }
    if param.status.is_none() {
        return Err(Error::Validation(ERR_AUDIT_STATUS_REQUIRED.into()));
    }
    Ok(())
}

/// 公开/隐藏参数校验（Java `@Validated BuyerShowCommentShowOrHideParam`）。
pub fn validate_show_or_hide_param(param: &BuyerShowCommentShowOrHideParam) -> Result<()> {
    if param.buyer_show_comment_id.is_none() {
        return Err(Error::Validation(ERR_COMMENT_ID_REQUIRED.into()));
    }
    if param.is_show.is_none() {
        return Err(Error::Validation(ERR_IS_SHOW_REQUIRED.into()));
    }
    Ok(())
}

/// 置顶参数校验（Java `@Validated BuyerShowCommentIsTopParam`）。
pub fn validate_is_top_param(param: &BuyerShowCommentIsTopParam) -> Result<()> {
    if param.buyer_show_comment_id.is_none() {
        return Err(Error::Validation(ERR_COMMENT_ID_REQUIRED.into()));
    }
    if param.is_top.is_none() {
        return Err(Error::Validation(ERR_IS_TOP_REQUIRED.into()));
    }
    Ok(())
}

/// 批量参数校验（Java `@RequestBody BatchDTO`，服务层方法**未加** `@Validated`，
/// 但 \"BatchDTO\" 的字段注解在 Spring MVC 绑定阶段即生效）。
///
/// 校验顺序与 Spring 的字段声明顺序一致：\"type\"（`@NotNull`）→ \"ids\"（`@NotEmpty`）。
pub fn validate_batch(batch: &BatchDTO) -> Result<()> {
    if batch.type_is_null() {
        return Err(Error::Validation(ERR_OPERATION_TYPE_REQUIRED.into()));
    }
    if batch.ids_is_empty() {
        return Err(Error::Validation(ERR_UNSELECTED_ITEM.into()));
    }
    Ok(())
}

/// 审核写入的 \"is_show\"（Java `param.getStatus() == 1 $1 Constants.YES : Constants.NO`）。
#[must_use]
pub fn audit_is_show(status: i32) -> i32 {
    if status == 1 {
        1
    } else {
        0
    }
}

/// 去重后的会员ID（保持首次出现顺序，仅 `user_id > 0`）。
///
/// 对应 Java \"list\" 中的 `filter(userId -> userId > 0)`。
#[must_use]
pub fn distinct_user_ids(comments: &[BuyerShowComment]) -> Vec<i32> {
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for c in comments {
        if c.user_id > 0 && seen.insert(c.user_id) {
            out.push(c.user_id);
        }
    }
    out
}

// ——————————————————————————————————————————————————————————————
// 服务
// ——————————————————————————————————————————————————————————————

pub struct BuyerShowCommentService {
    repos: Arc<Repositories>,
}

impl BuyerShowCommentService {
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    // ————————————————————— 列表 —————————————————————

    /// 买家秀评论分页（对应 Java `BuyerShowCommentController.list` /
    /// `BuyerShowCommentServiceImpl#list`）。
    ///
    /// 与原 Java 一致：当前页无数据时直接返回空分页（Java `new Page<>()`，
    /// 因此该分支的 \"total\" 也归零）。
    pub async fn list(&self, q: &BuyerShowCommentListQuery) -> Result<PageResult<BuyerShowCommentPageVO>> {
        let page = q.page_query();
        let (total, comments) = self.repos.marketing.buyer_show_comment.paginate_comments(q).await?;
        if comments.is_empty() {
            return Ok(PageResult::empty(&page));
        }

        // 会员信息：Java `userService.lambdaQuery().in(User::getUserId, userIds).list()`
        // 后收集为 Map；`user_id <= 0`（系统/虚拟用户）不查库。
        let user_ids = distinct_user_ids(&comments);
        let user_map = self.repos.marketing.buyer_show_comment.find_user_names(&user_ids).await?;

        let list: Vec<BuyerShowCommentPageVO> = comments
            .iter()
            .map(|c| to_page_vo(c, &user_map))
            .collect();
        Ok(PageResult::new(total, list, &page))
    }

    // ————————————————————— 审核 —————————————————————

    /// 审核（对应 Java `BuyerShowCommentController.audit` /
    /// `BuyerShowCommentServiceImpl#audit`）。
    ///
    /// 语义：按主键把 \"status\" 写为入参值，\"is_show\" 由 `status == 1` 推导
    /// （\"1\" → 公开，其余 → 隐藏）。Java 不校验 \"status\" 取值范围、也不检查
    /// 更新行数，本实现保持一致。
    pub async fn audit(&self, param: &BuyerShowCommentAuditParam) -> Result<()> {
        validate_audit_param(param)?;
        let id = param.buyer_show_comment_id.unwrap_or_default();
        let status = param.status.unwrap_or_default();
        let affected = self.repos.marketing.buyer_show_comment.audit(id, status).await?;
        if affected == 0 {
            // Java 静默成功；这里保留日志便于运维排查「审核了不存在的评论」
            tracing::warn!("买家秀评论审核未命中任何行: buyer_show_comment_id={id}");
        }
        Ok(())
    }

    // ————————————————————— 批量审核 —————————————————————

    /// 批量操作（对应 Java `BuyerShowCommentController.batch` /
    /// `BuyerShowCommentServiceImpl#batch`）。
    ///
    /// - `type == "audit"`：按 `ids` 批量写 `status = value` 与
    ///   `is_show = (value == 1 ? 1 : 0)`，返回 `true`；
    /// - 其余 \"type\"：Java 直接 `return false`，**无任何副作用**。
    pub async fn batch(&self, batch: &BatchDTO) -> Result<bool> {
        validate_batch(batch)?;
        if !batch.is_audit() {
            return Ok(false);
        }
        let value = batch.value_or_zero();
        let ids = batch.ids.as_deref().unwrap_or(&[]);
        let affected = self.repos.marketing.buyer_show_comment.batch_audit(ids, value).await?;
        tracing::info!("买家秀评论批量审核: ids={}, status={value}, 影响 {affected} 行", ids.len());
        Ok(true)
    }

    // ————————————————————— 公开 / 隐藏 —————————————————————

    /// 公开 / 隐藏（对应 Java `BuyerShowCommentController.showOrHide` /
    /// `BuyerShowCommentServiceImpl#showOrHide`）：仅写 `is_show`。
    pub async fn show_or_hide(&self, param: &BuyerShowCommentShowOrHideParam) -> Result<()> {
        validate_show_or_hide_param(param)?;
        let id = param.buyer_show_comment_id.unwrap_or_default();
        let is_show = param.is_show.unwrap_or_default();
        let affected = self.repos.marketing.buyer_show_comment.set_show(id, is_show).await?;
        if affected == 0 {
            tracing::warn!("买家秀评论公开/隐藏未命中任何行: buyer_show_comment_id={id}");
        }
        Ok(())
    }

    // ————————————————————— 置顶 —————————————————————

    /// 置顶 / 取消置顶（对应 Java `BuyerShowCommentController.isTop` /
    /// `BuyerShowCommentServiceImpl#isTop`）：仅写 `is_top`。
    pub async fn is_top(&self, param: &BuyerShowCommentIsTopParam) -> Result<()> {
        validate_is_top_param(param)?;
        let id = param.buyer_show_comment_id.unwrap_or_default();
        let is_top = param.is_top.unwrap_or_default();
        let affected = self.repos.marketing.buyer_show_comment.set_top(id, is_top).await?;
        if affected == 0 {
            tracing::warn!("买家秀评论置顶未命中任何行: buyer_show_comment_id={id}");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn comment(id: i32, user_id: i32, status: i32, is_show: i32, is_top: i32) -> BuyerShowComment {
        BuyerShowComment {
            buyer_show_comment_id: id,
            parent_id: 0,
            buyer_show_id: 3,
            status,
            is_top,
            is_show,
            user_id,
            user_name: if user_id == 0 {
                "官方".into()
            } else {
                "tig_user".into()
            },
            user_photo: String::new(),
            content: Some("很好".into()),
            add_time: 1_700_000_000,
            comment_pic: None,
        }
    }

    // ————————————————— 状态文案 —————————————————

    #[test]
    fn status_text_matches_java_ternary() {
        assert_eq!(status_text(0, 0), "待审核");
        // 待审核优先级最高：即使 is_show 已被写为 1，仍显示「待审核」
        assert_eq!(status_text(0, 1), "待审核");
        assert_eq!(status_text(1, 1), "公开");
        assert_eq!(status_text(1, 0), "隐藏");
        assert_eq!(status_text(2, 0), "隐藏");
        // Java 的 else 分支对任意非 1 的 is_show 都返回「隐藏」
        assert_eq!(status_text(1, 2), "隐藏");
    }

    // ————————————————— 发布人文案 —————————————————

    #[test]
    fn user_name_appends_type_suffix() {
        let user_map = HashMap::new();
        // 系统评论（userId == 0）
        assert_eq!(
            build_user_name(&comment(1, 0, 1, 1, 0), &user_map),
            "官方(系统)"
        );
        // 会员评论（userId > 0）
        assert_eq!(
            build_user_name(&comment(2, 9, 1, 1, 0), &user_map),
            "tig_user(客户)"
        );
    }

    #[test]
    fn user_name_follows_java_not_the_member_nickname() {
        // Java 把 `user.getName()` 赋给了实体、随后被 VO 的 userName 覆盖来源覆盖，
        // 因此会员昵称**不会**出现在返回里；此处固定该行为，防止"顺手修好"导致
        // 与线上返回不一致。
        let mut map = HashMap::new();
        map.insert(9, "会员昵称".to_string());
        assert_eq!(
            build_user_name(&comment(2, 9, 1, 1, 0), &map),
            "tig_user(客户)"
        );
    }

    #[test]
    fn user_type_suffix_only_system_for_zero() {
        assert_eq!(user_type_suffix(0), "(系统)");
        assert_eq!(user_type_suffix(1), "(客户)");
        // 非真实用户为 0，其余（含负数等异常数据）一律按客户处理
        assert_eq!(user_type_suffix(-1), "(客户)");
    }

    #[test]
    fn user_display_name_prefers_nickname() {
        let mut u = User {
            user_id: 1,
            username: "u1".into(),
            nickname: Some("昵称".into()),
            ..Default::default()
        };
        assert_eq!(user_display_name(&u), "昵称");
        u.nickname = Some("   ".into());
        assert_eq!(user_display_name(&u), "u1");
        u.nickname = None;
        assert_eq!(user_display_name(&u), "u1");
    }

    // ————————————————— 时间格式化 —————————————————

    #[test]
    fn fmt_time_zero_is_empty_string() {
        assert_eq!(fmt_time(0), "");
        assert_eq!(fmt_time(1_700_000_000).len(), 19);
    }

    // ————————————————— VO 组装 —————————————————

    #[test]
    fn page_vo_maps_all_fields() {
        let vo = to_page_vo(&comment(7, 5, 1, 1, 1), &HashMap::new());
        assert_eq!(vo.buyer_show_comment_id, 7);
        assert_eq!(vo.buyer_show_id, 3);
        assert_eq!(vo.is_top, 1);
        assert_eq!(vo.status, 1);
        assert_eq!(vo.is_show, 1);
        assert_eq!(vo.status_text, "公开");
        assert_eq!(vo.user_name, "tig_user(客户)");
        assert_eq!(vo.content.as_deref(), Some("很好"));
        assert_eq!(vo.add_time.len(), 19);
    }

    #[test]
    fn page_vo_keeps_null_content() {
        let mut c = comment(7, 0, 0, 0, 0);
        c.content = None;
        let vo = to_page_vo(&c, &HashMap::new());
        assert_eq!(vo.content, None);
        assert_eq!(vo.status_text, "待审核");
    }

    // ————————————————— 参数校验 —————————————————

    fn audit_param(id: Option<i32>, status: Option<i32>) -> BuyerShowCommentAuditParam {
        BuyerShowCommentAuditParam {
            buyer_show_comment_id: id,
            status,
        }
    }

    #[test]
    fn audit_param_requires_both_fields() {
        let err = validate_audit_param(&audit_param(None, Some(1))).unwrap_err();
        assert_eq!(err.to_string(), format!("参数校验失败: {ERR_COMMENT_ID_REQUIRED}"));

        let err = validate_audit_param(&audit_param(Some(1), None)).unwrap_err();
        assert_eq!(err.to_string(), format!("参数校验失败: {ERR_AUDIT_STATUS_REQUIRED}"));

        // Java `@NotNull` 只管非空，不校验取值域
        assert!(validate_audit_param(&audit_param(Some(1), Some(99))).is_ok());
    }

    #[test]
    fn show_or_hide_param_requires_both_fields() {
        let err = validate_show_or_hide_param(&BuyerShowCommentShowOrHideParam {
            buyer_show_comment_id: None,
            is_show: Some(1),
        })
        .unwrap_err();
        assert_eq!(err.to_string(), format!("参数校验失败: {ERR_COMMENT_ID_REQUIRED}"));

        let err = validate_show_or_hide_param(&BuyerShowCommentShowOrHideParam {
            buyer_show_comment_id: Some(1),
            is_show: None,
        })
        .unwrap_err();
        // Java 该字段的消息文案沿用了「审核状态不能为空」
        assert_eq!(err.to_string(), format!("参数校验失败: {ERR_IS_SHOW_REQUIRED}"));

        assert!(validate_show_or_hide_param(&BuyerShowCommentShowOrHideParam {
            buyer_show_comment_id: Some(1),
            is_show: Some(0),
        })
        .is_ok());
    }

    #[test]
    fn is_top_param_requires_both_fields() {
        assert!(validate_is_top_param(&BuyerShowCommentIsTopParam {
            buyer_show_comment_id: None,
            is_top: Some(1)
        })
        .is_err());
        assert!(validate_is_top_param(&BuyerShowCommentIsTopParam {
            buyer_show_comment_id: Some(1),
            is_top: None
        })
        .is_err());
        assert!(validate_is_top_param(&BuyerShowCommentIsTopParam {
            buyer_show_comment_id: Some(1),
            is_top: Some(1)
        })
        .is_ok());
    }

    #[test]
    fn audit_is_show_follows_status() {
        assert_eq!(audit_is_show(1), 1);
        assert_eq!(audit_is_show(0), 0);
        assert_eq!(audit_is_show(2), 0);
        // Java 无取值范围校验：只有恰好等于 1 才是公开
        assert_eq!(audit_is_show(-1), 0);
    }

    // ————————————————— 批量参数 —————————————————

    fn batch(ids: Option<Vec<i32>>, ty: Option<&str>, value: Option<i32>) -> BatchDTO {
        BatchDTO {
            ids,
            r#type: ty.map(str::to_string),
            value,
            data: None,
        }
    }

    #[test]
    fn batch_requires_type_then_ids() {
        // 先校验 type（Java 字段声明顺序：ids 在前但 `@NotNull` 在 type 上，
        // Spring 两个都报；这里固定为「type 优先」的确定性顺序）
        let err = validate_batch(&batch(None, None, None)).unwrap_err();
        assert_eq!(
            err.to_string(),
            format!("参数校验失败: {ERR_OPERATION_TYPE_REQUIRED}")
        );

        let err = validate_batch(&batch(None, Some("audit"), None)).unwrap_err();
        assert_eq!(err.to_string(), format!("参数校验失败: {ERR_UNSELECTED_ITEM}"));

        let err = validate_batch(&batch(Some(vec![]), Some("audit"), None)).unwrap_err();
        assert_eq!(err.to_string(), format!("参数校验失败: {ERR_UNSELECTED_ITEM}"));

        assert!(validate_batch(&batch(Some(vec![1, 2]), Some("audit"), Some(1))).is_ok());
        // 非 audit 的 type 也通过校验（由服务层决定无副作用）
        assert!(validate_batch(&batch(Some(vec![1]), Some("other"), None)).is_ok());
    }

    #[test]
    fn batch_audit_flag_only_for_exact_audit() {
        assert!(batch(Some(vec![1]), Some("audit"), None).is_audit());
        // Java `"audit".equals(type)`：大小写敏感
        assert!(!batch(Some(vec![1]), Some("Audit"), None).is_audit());
        assert!(!batch(Some(vec![1]), Some("del"), None).is_audit());
        assert!(!batch(Some(vec![1]), None, None).is_audit());
    }

    #[test]
    fn batch_value_defaults_to_zero_like_java_int() {
        assert_eq!(batch(Some(vec![1]), Some("audit"), None).value_or_zero(), 0);
        assert_eq!(
            batch(Some(vec![1]), Some("audit"), Some(2)).value_or_zero(),
            2
        );
        // value = 0 → is_show 落库为 0（隐藏），status 落库为 0（待审核）
        assert_eq!(audit_is_show(batch(Some(vec![1]), Some("audit"), None).value_or_zero()), 0);
    }

    // ————————————————— 会员ID收集 —————————————————

    #[test]
    fn distinct_user_ids_skips_non_positive_and_dupes() {
        let comments = vec![
            comment(1, 0, 1, 1, 0), // 系统用户，跳过
            comment(2, 5, 1, 1, 0),
            comment(3, 5, 1, 1, 0), // 重复，去重
            comment(4, 6, 1, 1, 0),
        ];
        assert_eq!(distinct_user_ids(&comments), vec![5, 6]);
        assert!(distinct_user_ids(&[]).is_empty());
    }
}"