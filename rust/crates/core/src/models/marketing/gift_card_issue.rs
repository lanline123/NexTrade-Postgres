//! 礼品卡发放域（marketing::gift_card_issue）DTO / Query / VO / 纯函数。
//!
//! 礼品卡补齐 P1（发放闭环）：
//! - 后台定向发放：指定会员 USER / 会员等级 RANK / 用户标签 TAG / Excel 名单 EXCEL，
//!   目标解析口径复用优惠券手动发放（`marketing::coupon_manage`）；
//! - 发放 = 从卡组现有可用卡里分配（不自动制卡），库存不足整单拦截；
//! - 前台「我的礼品卡」：归属 = `claimed_user_id` 优先、其次 `user_id`。
//!
//! 表：`gift_card_issue_batch` / `gift_card_issue_log` / `gift_card_usage_log`
//! （迁移 `rust/deploy/pg/2026-11-18_gift_card_issue.sql`）。
//! 批次与流水只记 `gift_cards_id`，卡号 / 卡密不进新接口响应（明细联表回填时脱敏）。

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

// ——————————————————————————————————————————————
// 常量（文案）
// ——————————————————————————————————————————————

/// 卡组 id 不能为空
pub const MSG_GROUP_ID_REQUIRED: &str = "卡组id不能为空";
/// 卡组不存在
pub const MSG_GROUP_NOT_EXISTS: &str = "卡组不存在";
/// 无权发放该礼品卡
pub const MSG_ISSUE_NO_PERMISSION: &str = "无权发放该礼品卡";
/// 发放模式不正确
pub const MSG_ISSUE_MODE_INVALID: &str = "发放模式不正确";
/// 发放模式不能为空
pub const MSG_ISSUE_TYPE_REQUIRED: &str = "发放模式不能为空";
/// 请选择会员
pub const MSG_ISSUE_MEMBER_REQUIRED: &str = "请选择会员";
/// 手动发放暂不支持混合选择
pub const MSG_ISSUE_MIXED_UNSUPPORTED: &str = "手动发放暂不支持混合选择";
/// 请选择会员等级
pub const MSG_ISSUE_RANK_REQUIRED: &str = "请选择会员等级";
/// 请选择会员标签
pub const MSG_ISSUE_TAG_REQUIRED: &str = "请选择会员标签";
/// 没有可发放的会员
pub const MSG_NO_VALID_MEMBER: &str = "没有可发放的会员";
/// Excel 发卡：卡号/卡密不匹配或卡不存在（失败明细原因）
pub const MSG_EXCEL_CARD_NOT_FOUND: &str = "卡号或卡密不正确，卡不存在";
/// Excel 发卡：卡已使用/已发放/不可用（失败明细原因）
pub const MSG_EXCEL_CARD_USED: &str = "该卡已使用或已发放";
/// Excel 发卡：同一卡号在文件内重复（失败明细原因）
pub const MSG_EXCEL_CARD_DUPLICATE: &str = "文件内卡号重复";
/// Excel 发卡：行缺少会员或卡信息（失败明细原因）
pub const MSG_EXCEL_ROW_INCOMPLETE: &str = "会员名/手机号与卡号/卡密不能为空";
/// 会员不存在（失败明细原因）
pub const MSG_MEMBER_NOT_EXIST: &str = "会员不存在";
/// 会员已禁用（失败明细原因）
pub const MSG_MEMBER_DISABLED: &str = "会员已禁用";
/// 名单内重复（Excel 失败明细原因）
pub const MSG_EXCEL_DUPLICATE: &str = "名单内重复";
/// 名单行内容为空（Excel 失败明细原因）
pub const MSG_EXCEL_ROW_EMPTY: &str = "名单行内容为空";
/// 礼品卡不存在（我的卡详情归属校验未过时同口径，不泄露他人卡存在性）
pub const MSG_CARD_NOT_EXISTS: &str = "礼品卡不存在";
/// 请先登录
pub const MSG_LOGIN_REQUIRED: &str = "请先登录";

/// 发放方式：USER 指定会员。
pub const ISSUE_TYPE_USER: &str = "USER";
/// 发放方式：RANK 会员等级。
pub const ISSUE_TYPE_RANK: &str = "RANK";
/// 发放方式：TAG 用户标签。
pub const ISSUE_TYPE_TAG: &str = "TAG";
/// 发放方式：EXCEL 名单导入。
pub const ISSUE_TYPE_EXCEL: &str = "EXCEL";

/// 每人发卡数（D3 定案：一人一卡，原版 issue payload 无每人张数字段）。
pub const CARDS_PER_USER: i32 = 1;

// ——————————————————————————————————————————————
// 纯函数
// ——————————————————————————————————————————————

/// 归一化发放模式（trim + 大写 + 枚举校验，含 EXCEL）。
#[must_use]
pub fn normalize_issue_type(raw: &str) -> Option<&'static str> {
    match raw.trim().to_uppercase().as_str() {
        "USER" => Some(ISSUE_TYPE_USER),
        "RANK" => Some(ISSUE_TYPE_RANK),
        "TAG" => Some(ISSUE_TYPE_TAG),
        "EXCEL" => Some(ISSUE_TYPE_EXCEL),
        _ => None,
    }
}

/// 发放方式文案。
#[must_use]
pub fn issue_type_name(issue_type: &str) -> &'static str {
    match issue_type {
        ISSUE_TYPE_USER => "指定会员",
        ISSUE_TYPE_RANK => "会员等级",
        ISSUE_TYPE_TAG => "用户标签",
        ISSUE_TYPE_EXCEL => "Excel名单",
        _ => "未知",
    }
}

/// 需卡数 = 目标人数（一人一卡，D3 定案）。
#[must_use]
pub fn need_count(member_count: usize) -> i64 {
    member_count as i64
}

/// 库存不足文案（整单拦截口径）。
#[must_use]
pub fn insufficient_stock_message(available: i64, need: i64) -> String {
    format!("发放失败，礼品卡库存不足（可用 {available} 张，需发 {need} 张），请先补充制卡后重试")
}

/// 卡是否已过期（`expired_time == 0` ⇒ 永久；`status == 2` ⇒ 已过期）。
#[must_use]
pub fn card_is_expired(status: Option<i32>, expired_time: i32, now: i32) -> bool {
    status == Some(2) || (expired_time != 0 && now > expired_time)
}

/// 派生状态（我的礼品卡）：返回 `(状态文案, 是否有效)`。
///
/// 优先级：已用完/已使用（status=1）> 已过期 > 部分使用（储值卡 0 < balance < face）> 未使用。
#[must_use]
pub fn derive_card_state(
    status: Option<i32>,
    expired_time: i32,
    now: i32,
    group_type: i32,
    face_amount: Option<Decimal>,
    balance: Option<Decimal>,
) -> (&'static str, bool) {
    if status == Some(1) {
        return if group_type == 1 { ("已用完", false) } else { ("已使用", false) };
    }
    if card_is_expired(status, expired_time, now) {
        return ("已过期", false);
    }
    if group_type == 1 {
        if let (Some(face), Some(balance)) = (face_amount, balance) {
            if balance > Decimal::ZERO && balance < face {
                return ("部分使用", true);
            }
        }
    }
    ("未使用", true)
}

/// 卡来源文案：发放（有 issue_log）> 购买（有 gift_card_order）> 转赠领取。
#[must_use]
pub fn card_source_text(has_issue: bool, has_order: bool) -> &'static str {
    if has_issue {
        "发放"
    } else if has_order {
        "购买"
    } else {
        "转赠领取"
    }
}

/// 消费流水类型文案（1 结算抵扣 / 2 订单取消或退款返还）。
/// D4 定案：储值卡不设「兑换入余额」路径，usage_log 不设 type=3。
#[must_use]
pub fn usage_type_name(usage_type: i32) -> &'static str {
    match usage_type {
        1 => "结算抵扣",
        2 => "退款返还",
        _ => "未知",
    }
}

/// Excel 发卡行（原版模板列序：会员名 / 手机号 / 卡号 / 卡密 / 备注）。
///
/// 来源标注：**原版有** — giftmall Admin `IssueExcel` 组件说明原文：
/// 「请先下载模板，按会员名、手机号、卡号、卡密、备注填写。卡号卡密必须
/// 已存在且未使用。」即逐行把指定的已存在卡发给指定会员，不是名单自动分配。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct IssueExcelRow {
    /// 会员名（与手机号二选一必填；匹配优先级 手机号 > 会员名）
    pub member_name: String,
    /// 手机号（匹配优先级最高）
    pub mobile: String,
    /// 卡号（必须已存在且未使用，与卡密成对校验）
    pub card_number: String,
    /// 卡密
    pub card_password: String,
    /// 备注（写入批次 remark 参考，不参与匹配）
    pub remark: String,
}

/// 解析一行 Excel 发卡数据（已 trim 的单元格）：
/// 会员名/手机号至少一项非空，且卡号、卡密均非空才成行。
#[must_use]
pub fn parse_issue_excel_row(cells: &[String]) -> Option<IssueExcelRow> {
    let cell = |idx: usize| cells.get(idx).map(|s| s.trim().to_string()).unwrap_or_default();
    let row = IssueExcelRow {
        member_name: cell(0),
        mobile: cell(1),
        card_number: cell(2),
        card_password: cell(3),
        remark: cell(4),
    };
    if (row.member_name.is_empty() && row.mobile.is_empty())
        || row.card_number.is_empty()
        || row.card_password.is_empty()
    {
        return None;
    }
    Some(row)
}

// ——————————————————————————————————————————————
// 入参
// ——————————————————————————————————————————————

/// 定向发放参数（preview / send 共用）。
///
/// 来源标注：**原版有·我方改口径** — 原版为单次 `POST promotion/giftCards/issue`，
/// 我方加 preview 预检（库存/失败明细先行），发放语义与原版一致
/// （USER/TAG/RANK 三对象、一人一卡、批次+明细留痕）。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct GiftCardIssueParam {
    /// 卡组 id（@NotNull）
    pub card_groups_id: i32,
    /// 发放模式 USER/RANK/TAG（@NotBlank）
    pub issue_type: String,
    pub user_ids: Option<Vec<i32>>,
    pub rank_ids: Option<Vec<i32>>,
    pub tag_ids: Option<Vec<i64>>,
    /// 标签匹配：0 满足任一（默认），1 满足全部
    pub tag_match_mode: Option<i32>,
    pub remark: Option<String>,
}

/// 发放批次分页查询。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct GiftCardIssueBatchPageQuery {
    pub card_groups_id: Option<i32>,
    pub issue_type: Option<String>,
    /// 发放时间起（秒级时间戳）
    pub start_time: Option<i64>,
    /// 发放时间止（秒级时间戳）
    pub end_time: Option<i64>,
    pub page: Option<u64>,
    pub size: Option<u64>,
}

/// 发放明细分页查询。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct GiftCardIssueLogPageQuery {
    pub batch_id: Option<i64>,
    pub card_groups_id: Option<i32>,
    pub user_id: Option<i32>,
    pub page: Option<u64>,
    pub size: Option<u64>,
}

/// 我的礼品卡分页查询（前台登录态）。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct MyGiftCardsQuery {
    /// 卡组类型：1 储值卡 / 2 兑换卡（空 = 全部）
    pub r#type: Option<i32>,
    /// 状态筛选：valid 有效 / invalid 失效（空 = 全部）
    pub state: Option<String>,
    pub page: Option<u64>,
    pub size: Option<u64>,
}

// ——————————————————————————————————————————————
// 出参
// ——————————————————————————————————————————————

/// 发放失败明细（逐会员）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct GiftCardIssueFailDetailVO {
    pub user_id: i32,
    pub member_name: String,
    pub mobile: String,
    pub reason: String,
}

/// 发放预览。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct GiftCardIssuePreviewVO {
    pub card_groups_id: i32,
    pub issue_type: String,
    /// 目标人数（解析后有效会员数）
    pub member_count: i32,
    /// 需卡数 = member_count（一人一卡）
    pub need_count: i64,
    /// 卡组当前可用库存
    pub available_stock: i64,
    /// 库存缺口（0 = 不缺）
    pub shortage: i64,
    /// 是否可发（有有效会员且库存足够）
    pub can_issue: bool,
    pub fail_count: i32,
    pub fail_details: Vec<GiftCardIssueFailDetailVO>,
}

/// 发放结果。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct GiftCardIssueSendVO {
    pub batch_id: i64,
    /// 成功人数
    pub success_count: i32,
    /// 成功发放卡数
    pub card_count: i32,
    pub fail_count: i32,
    pub fail_details: Vec<GiftCardIssueFailDetailVO>,
}

/// Excel 名单逐行失败明细。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct GiftCardExcelFailDetailVO {
    /// Excel 行号（含表头，第 1 行为表头，数据从第 2 行起）
    pub row: i32,
    /// 原值（命中的匹配键原值）
    pub raw_value: String,
    pub reason: String,
}

/// Excel 发卡结果（原版 issueByExcel 单次提交口径：逐行发放，
/// 失败行记录原因、其余继续；不做预览两段式）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct GiftCardIssueByExcelVO {
    pub card_groups_id: i32,
    /// 发放批次 id（全部失败时为 0，不产生批次）
    pub batch_id: i64,
    /// 名单总行数（不含表头，不含整行空白）
    pub total_rows: i32,
    pub success_count: i32,
    pub fail_count: i32,
    pub fail_details: Vec<GiftCardExcelFailDetailVO>,
}

/// 发放批次 VO。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct GiftCardIssueBatchVO {
    pub id: i64,
    pub card_groups_id: i32,
    pub card_group_name: Option<String>,
    pub shop_id: i32,
    pub issue_type: String,
    pub issue_type_name: String,
    pub target_desc: serde_json::Value,
    pub cards_per_user: i32,
    pub target_count: i32,
    pub success_count: i32,
    pub fail_count: i32,
    pub admin_id: i32,
    pub remark: String,
    pub add_time: i64,
}

/// 发放明细 VO（卡号脱敏）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct GiftCardIssueLogVO {
    pub id: i64,
    pub batch_id: i64,
    pub card_groups_id: i32,
    pub card_group_name: Option<String>,
    pub gift_cards_id: i32,
    /// 卡号（脱敏）
    pub card_number: Option<String>,
    pub user_id: i32,
    pub username: Option<String>,
    pub nickname: Option<String>,
    pub mobile: Option<String>,
    pub issue_time: i64,
}

/// 我的礼品卡 VO。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct MyGiftCardVO {
    pub gift_cards_id: i32,
    pub card_groups_id: i32,
    pub card_group_name: Option<String>,
    pub image_url: Option<String>,
    /// 1 储值卡 / 2 兑换卡
    pub group_type: i32,
    pub group_type_name: String,
    /// 1 电子卡 / 2 实体卡
    pub card_type: i32,
    /// 卡号（脱敏）
    pub card_number: Option<String>,
    pub gift_card_sn: Option<String>,
    pub face_amount: Option<Decimal>,
    /// 储值卡剩余余额（兑换卡为 null）
    pub balance: Option<Decimal>,
    pub status: Option<i32>,
    /// 派生状态文案：未使用 / 部分使用 / 已用完 / 已使用 / 已过期
    pub state_text: String,
    pub is_valid: bool,
    /// 来源文案：购买 / 发放 / 转赠领取
    pub source_text: String,
    pub expired_time: i32,
    pub used_time: Option<i32>,
    pub add_time: Option<i32>,
}

/// 消费流水 VO。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct GiftCardUsageLogVO {
    pub id: i64,
    pub gift_cards_id: i32,
    pub user_id: i32,
    pub order_id: i32,
    pub r#type: i32,
    pub type_name: String,
    pub amount: Decimal,
    pub balance_before: Option<Decimal>,
    pub balance_after: Option<Decimal>,
    pub add_time: i64,
}

/// 我的礼品卡详情 VO。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct MyGiftCardDetailVO {
    #[serde(flatten)]
    pub card: MyGiftCardVO,
    /// 消费记录（gift_card_usage_log：结算抵扣/返还流水）
    pub usage_logs: Vec<GiftCardUsageLogVO>,
    /// 余额流水（原版详情字段名 `balanceLogs`，与 usage_logs 同数据，
    /// 供按原版口径消费的前端直接读取）
    pub balance_logs: Vec<GiftCardUsageLogVO>,
    /// 兑换卡的兑换订单 id（gift_card_order / order_item 关联）
    pub exchange_order_ids: Vec<i32>,
    /// 卡组兑换模式（1 固定套餐 / 2 多件任选；详情页「去使用」分流依据）
    pub exchange_mode: i32,
    /// 任选必选件数（exchange_mode=2 时有效）
    pub select_count: i32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn issue_type_normalize_and_names() {
        assert_eq!(normalize_issue_type(" user "), Some(ISSUE_TYPE_USER));
        assert_eq!(normalize_issue_type("rank"), Some(ISSUE_TYPE_RANK));
        assert_eq!(normalize_issue_type("TAG"), Some(ISSUE_TYPE_TAG));
        assert_eq!(normalize_issue_type("excel"), Some(ISSUE_TYPE_EXCEL));
        assert_eq!(normalize_issue_type("bad"), None);
        assert_eq!(issue_type_name(ISSUE_TYPE_EXCEL), "Excel名单");
        assert_eq!(issue_type_name("?"), "未知");
    }

    #[test]
    fn need_count_is_one_card_per_member() {
        assert_eq!(CARDS_PER_USER, 1);
        assert_eq!(need_count(3), 3);
        assert_eq!(need_count(0), 0);
        assert_eq!(
            insufficient_stock_message(2, 3),
            "发放失败，礼品卡库存不足（可用 2 张，需发 3 张），请先补充制卡后重试"
        );
    }

    #[test]
    fn derive_state_priority() {
        let face = Some(Decimal::new(10000, 2));
        // 未使用
        assert_eq!(
            derive_card_state(Some(0), 0, 100, 1, face, face),
            ("未使用", true)
        );
        // 部分使用
        assert_eq!(
            derive_card_state(Some(0), 0, 100, 1, face, Some(Decimal::new(4000, 2))),
            ("部分使用", true)
        );
        // 已用完优先于过期
        assert_eq!(
            derive_card_state(Some(1), 10, 100, 1, face, Some(Decimal::ZERO)),
            ("已用完", false)
        );
        // 兑换卡已使用
        assert_eq!(
            derive_card_state(Some(1), 0, 100, 2, None, None),
            ("已使用", false)
        );
        // 已过期：时间过期 / status=2
        assert_eq!(
            derive_card_state(Some(0), 50, 100, 1, face, face),
            ("已过期", false)
        );
        assert_eq!(
            derive_card_state(Some(2), 0, 100, 2, None, None),
            ("已过期", false)
        );
    }

    #[test]
    fn source_and_usage_names() {
        assert_eq!(card_source_text(true, true), "发放");
        assert_eq!(card_source_text(false, true), "购买");
        assert_eq!(card_source_text(false, false), "转赠领取");
        assert_eq!(usage_type_name(1), "结算抵扣");
        assert_eq!(usage_type_name(2), "退款返还");
        // D4：不设 type=3（兑换入余额已下线）
        assert_eq!(usage_type_name(3), "未知");
    }

    #[test]
    fn excel_row_uses_original_columns() {
        // 原版列序：会员名 / 手机号 / 卡号 / 卡密 / 备注
        let row = parse_issue_excel_row(&[
            " bob ".into(),
            "13800000000".into(),
            "GC20260101".into(),
            "abcd1234".into(),
            "年会福利".into(),
        ])
        .unwrap();
        assert_eq!(row.member_name, "bob");
        assert_eq!(row.mobile, "13800000000");
        assert_eq!(row.card_number, "GC20260101");
        assert_eq!(row.card_password, "abcd1234");
        assert_eq!(row.remark, "年会福利");
        // 会员名与手机号可二选一；卡号卡密必填
        assert!(parse_issue_excel_row(&[
            String::new(),
            "138".into(),
            "GC1".into(),
            "pw".into()
        ])
        .is_some());
        assert!(parse_issue_excel_row(&[
            String::new(),
            String::new(),
            "GC1".into(),
            "pw".into()
        ])
        .is_none());
        assert!(parse_issue_excel_row(&["bob".into(), String::new(), String::new(), "pw".into()])
            .is_none());
        assert!(parse_issue_excel_row(&[]).is_none());
    }

    #[test]
    fn param_and_query_camel_case() {
        let p: GiftCardIssueParam = serde_json::from_str(
            r#"{"cardGroupsId":3,"issueType":"TAG","tagIds":[1,2],"tagMatchMode":1}"#,
        )
        .unwrap();
        assert_eq!(p.card_groups_id, 3);
        assert_eq!(p.tag_ids, Some(vec![1, 2]));
        let q: MyGiftCardsQuery =
            serde_json::from_str(r#"{"type":1,"state":"valid","page":2}"#).unwrap();
        assert_eq!(q.r#type, Some(1));
        assert_eq!(q.state.as_deref(), Some("valid"));
    }
}
