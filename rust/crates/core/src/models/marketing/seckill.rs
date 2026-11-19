//! 秒杀域 DTO/VO。
//! 对应原 Java `dto.promotion.Seckill*` 与 `vo.promotion.Seckill*`。
//!
//! JSON 字段名保持 camelCase 与原前端接口一致。

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// 秒杀活动分页查询。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SeckillListQuery {
    pub keyword: Option<String>,
    pub shop_id: Option<i32>,
    pub page: Option<u64>,
    #[serde(rename = "pageSize")]
    #[serde(alias = "size")]
    pub page_size: Option<u64>,
}

/// 秒杀明细入参（创建/更新活动时一并提交）。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SeckillItemDTO {
    /// 更新场景下已有明细的主键；新增时为空
    pub rec_id: Option<i32>,
    #[serde(
        default,
        deserialize_with = "crate::models::serde_helpers::deserialize_opt_i32"
    )]
    pub product_id: Option<i32>,
    #[serde(
        default,
        deserialize_with = "crate::models::serde_helpers::deserialize_opt_i32"
    )]
    pub sku_id: Option<i32>,
    pub seckill_price: Option<Decimal>,
    #[serde(
        default,
        deserialize_with = "crate::models::serde_helpers::deserialize_opt_i32"
    )]
    pub seckill_stock: Option<i32>,
    #[serde(
        default,
        deserialize_with = "crate::models::serde_helpers::deserialize_opt_i32"
    )]
    pub seckill_start_time: Option<i32>,
    #[serde(
        default,
        deserialize_with = "crate::models::serde_helpers::deserialize_opt_i32"
    )]
    pub seckill_end_time: Option<i32>,
}

/// 创建秒杀活动请求。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SeckillCreateDTO {
    pub seckill_name: Option<String>,
    #[serde(
        default,
        deserialize_with = "crate::models::serde_helpers::deserialize_opt_i32"
    )]
    pub seckill_start_time: Option<i32>,
    #[serde(
        default,
        deserialize_with = "crate::models::serde_helpers::deserialize_opt_i32"
    )]
    pub seckill_end_time: Option<i32>,
    /// 每人限购数量（0 或空表示不限）
    #[serde(
        default,
        deserialize_with = "crate::models::serde_helpers::deserialize_opt_i32"
    )]
    pub seckill_limit_num: Option<i32>,
    #[serde(
        default,
        deserialize_with = "crate::models::serde_helpers::deserialize_opt_i32"
    )]
    pub product_id: Option<i32>,
    #[serde(
        default,
        deserialize_with = "crate::models::serde_helpers::deserialize_opt_i32"
    )]
    pub shop_id: Option<i32>,
    #[serde(default)]
    pub seckill_item: Vec<SeckillItemDTO>,
    /// 适用会员范围：0-全部会员，1-指定标记的会员（Java `SeckillCreateDTO.memberRange`）。
    /// 随活动写入 `promotion` 镜像行（Java `savePromotion`），此前未声明被 serde 静默丢弃。
    #[serde(
        default,
        deserialize_with = "crate::models::serde_helpers::deserialize_opt_i32"
    )]
    pub member_range: Option<i32>,
    /// 选择的会员标记列表（Java `memberMarks`），写入镜像行时按 Java
    /// `List.toString()` 落 `member_marks`。
    pub member_marks: Option<Vec<i32>>,
    /// 会员人群圈选（历史结构，Java `memberAudience`）；仅当 `sendUserTarget`
    /// 缺省时用于升维出镜像行的 `member_scope`。
    pub member_audience: Option<Value>,
    /// 参与人群（Java `sendUserTarget`，优先于 `memberAudience`），
    /// 经 [`resolve_member_scope`] 归一后落镜像行 `member_scope`。
    pub send_user_target: Option<Value>,
}

/// 更新秒杀活动请求（在创建字段基础上带 `seckillId`）。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SeckillUpdateDTO {
    pub seckill_id: i32,
    #[serde(flatten)]
    pub base: SeckillCreateDTO,
}

/// 秒杀活动 VO（管理端列表/详情）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SeckillVO {
    pub seckill_id: i32,
    pub seckill_name: String,
    pub seckill_start_time: i32,
    pub seckill_end_time: i32,
    pub seckill_limit_num: i32,
    pub product_id: i32,
    pub shop_id: i32,
    /// 明细列表
    pub seckill_item: Vec<SeckillItemVO>,
    /// 派生状态：1 未开始 2 进行中 3 已结束
    pub status: i32,
    pub status_name: String,
}

/// 秒杀明细 VO。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SeckillItemVO {
    pub rec_id: i32,
    pub seckill_id: i32,
    pub product_id: i32,
    pub sku_id: i32,
    pub seckill_price: Decimal,
    pub seckill_stock: i32,
    pub seckill_sales: i32,
    pub seckill_start_time: i32,
    pub seckill_end_time: i32,
}

/// 前台秒杀商品列表项（秒杀专区/装修组件用）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SeckillPromotionVO {
    pub rec_id: i32,
    pub seckill_id: i32,
    pub product_id: i32,
    pub sku_id: i32,
    /// 秒杀价
    pub seckill_price: Decimal,
    /// 剩余库存
    pub seckill_stock: i32,
    /// 已售数量
    pub seckill_sales: i32,
    pub seckill_start_time: i32,
    pub seckill_end_time: i32,
    /// 距开始/结束的剩余秒数（进行中为距结束，未开始为距开始）
    pub remain_seconds: i64,
    /// 是否已开始
    pub started: bool,
    /// 售罄进度百分比（0-100），用于前端进度条
    pub sold_percent: i32,
    /// 商品名称（前台卡片展示，Java 口径由商品表回填）
    pub product_name: String,
    /// 商品货号
    pub product_sn: String,
    /// 商品简介
    pub product_brief: String,
    /// 商品主图
    pub pic_url: String,
    /// 商品缩略图
    pub pic_thumb: String,
    /// 商品原价
    pub product_price: Decimal,
    /// 市场价
    pub market_price: Decimal,
    /// 商品类型（1 实物 / 2 虚拟 / 3 卡密 ...）
    pub product_type: Option<i32>,
}

/// 秒杀活动派生状态（非库列）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeckillStatus {
    /// 未开始
    NotStarted = 1,
    /// 进行中
    Running = 2,
    /// 已结束
    Finished = 3,
}

impl SeckillStatus {
    pub fn name(self) -> &'static str {
        match self {
            Self::NotStarted => "未开始",
            Self::Running => "进行中",
            Self::Finished => "已结束",
        }
    }

    /// 按时间窗推导状态。
    pub fn of(now: i32, start: i32, end: i32) -> Self {
        if now < start {
            Self::NotStarted
        } else if now > end {
            Self::Finished
        } else {
            Self::Running
        }
    }
}

impl TryFrom<i32> for SeckillStatus {
    type Error = String;

    fn try_from(v: i32) -> std::result::Result<Self, Self::Error> {
        match v {
            1 => Ok(Self::NotStarted),
            2 => Ok(Self::Running),
            3 => Ok(Self::Finished),
            other => Err(format!("非法秒杀状态: {other}")),
        }
    }
}

/// 人群范围（Java `SendUserTargetDTO.SCOPE_ALL`）：全部会员。
pub const MEMBER_SCOPE_ALL: i64 = 0;
/// 人群范围（Java `SendUserTargetDTO.SCOPE_PART`）：部分会员。
pub const MEMBER_SCOPE_PART: i64 = 1;

/// 历史人群类型（Java `MemberAudienceDTO.TYPE_RANK`）：按会员等级。
const AUDIENCE_TYPE_RANK: i64 = 1;
/// 历史人群类型（Java `MemberAudienceDTO.TYPE_TAG`）：按会员标签。
const AUDIENCE_TYPE_TAG: i64 = 2;

/// 读 JSON 对象中的整数键（缺失/非整数 → `None`）。
fn json_i64(v: &Value, key: &str) -> Option<i64> {
    v.get(key).and_then(Value::as_i64)
}

/// 读 JSON 对象中的 ID 数组（缺失/非数组 → 空；非整数元素跳过）。
fn json_id_list(v: &Value, key: &str) -> Vec<i64> {
    v.get(key)
        .and_then(Value::as_array)
        .map(|a| a.iter().filter_map(Value::as_i64).collect())
        .unwrap_or_default()
}

/// 标签匹配模式（Java `matchModeOrDefault`：仅 1 表示全匹配，其余归一为 0 任一匹配）。
fn json_match_mode(v: &Value) -> i64 {
    if json_i64(v, "tagMatchMode") == Some(1) {
        1
    } else {
        0
    }
}

/// 归一参与人群为 `promotion.member_scope` 落库值。
///
/// 等价 Java `MemberAudienceServiceImpl#resolveWriteScope` →
/// `normalizeExclusive` → `toScopeJson`（写侧口径，逐条对照源码）：
/// - `sendUserTarget` 优先；缺省时用 `memberAudience` 升维
///   （TYPE_ALL → 全部会员；TYPE_RANK → rankIds；TYPE_TAG → tagIds+匹配模式）；
///   两者皆缺 → `None`。
/// - 全部会员（scope 缺省或 0）→ `None`（Java：全部会员不落库，等价 memberRange=0）。
/// - 部分会员 → **互斥归一**：userIds / rankIds / tagIds 按此优先级只保留
///   第一个非空组；输出 `{"scope":1,...}`，仅含保留组（tagIds 组另带
///   `tagMatchMode`），`users` 明细永不落库。
///
/// 一处未复刻：Java `validateTarget` 对「部分会员但三组全空」直接抛错拒绝
/// 创建；我方创建校验未含此项，此处按 `toScopeJson` 的机械输出返回
/// `{"scope":1}`（人群匹配本身未迁移，该值当前无消费方按组判定）。
#[must_use]
pub fn resolve_member_scope(target: Option<&Value>, audience: Option<&Value>) -> Option<String> {
    let (scope, user_ids, rank_ids, tag_ids, tag_match_mode) = if let Some(t) = target {
        (
            json_i64(t, "scope").unwrap_or(MEMBER_SCOPE_ALL),
            json_id_list(t, "userIds"),
            json_id_list(t, "rankIds"),
            json_id_list(t, "tagIds"),
            json_match_mode(t),
        )
    } else if let Some(a) = audience {
        match json_i64(a, "audienceType").unwrap_or(MEMBER_SCOPE_ALL) {
            AUDIENCE_TYPE_RANK => (
                MEMBER_SCOPE_PART,
                Vec::new(),
                json_id_list(a, "rankIds"),
                Vec::new(),
                0,
            ),
            AUDIENCE_TYPE_TAG => (
                MEMBER_SCOPE_PART,
                Vec::new(),
                Vec::new(),
                json_id_list(a, "tagIds"),
                json_match_mode(a),
            ),
            _ => (MEMBER_SCOPE_ALL, Vec::new(), Vec::new(), Vec::new(), 0),
        }
    } else {
        return None;
    };
    if scope == MEMBER_SCOPE_ALL {
        return None;
    }
    let mut obj = serde_json::Map::new();
    obj.insert("scope".to_string(), Value::from(MEMBER_SCOPE_PART));
    if !user_ids.is_empty() {
        obj.insert("userIds".to_string(), Value::from(user_ids));
    } else if !rank_ids.is_empty() {
        obj.insert("rankIds".to_string(), Value::from(rank_ids));
    } else if !tag_ids.is_empty() {
        obj.insert("tagIds".to_string(), Value::from(tag_ids));
        obj.insert("tagMatchMode".to_string(), Value::from(tag_match_mode));
    }
    Some(Value::Object(obj).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_derivation() {
        // 未开始
        assert_eq!(SeckillStatus::of(100, 200, 300), SeckillStatus::NotStarted);
        // 进行中（边界：等于起止时间均算进行中）
        assert_eq!(SeckillStatus::of(200, 200, 300), SeckillStatus::Running);
        assert_eq!(SeckillStatus::of(300, 200, 300), SeckillStatus::Running);
        // 已结束
        assert_eq!(SeckillStatus::of(301, 200, 300), SeckillStatus::Finished);
    }

    #[test]
    fn status_names() {
        assert_eq!(SeckillStatus::Running.name(), "进行中");
        assert_eq!(SeckillStatus::try_from(3).unwrap().name(), "已结束");
        assert!(SeckillStatus::try_from(9).is_err());
    }

    #[test]
    fn create_dto_parses_camel_case() {
        let d: SeckillCreateDTO = serde_json::from_str(
            r#"{"seckillName":"618秒杀","seckillStartTime":100,"seckillEndTime":200,
                "seckillItem":[{"productId":9,"skuId":3,"seckillPrice":"9.90","seckillStock":50}]}"#,
        )
        .unwrap();
        assert_eq!(d.seckill_name.as_deref(), Some("618秒杀"));
        assert_eq!(d.seckill_item.len(), 1);
        assert_eq!(d.seckill_item[0].sku_id, Some(3));
        assert_eq!(d.seckill_item[0].seckill_stock, Some(50));
    }

    #[test]
    fn list_query_is_default_tolerant() {
        let q: SeckillListQuery = serde_json::from_str("{}").unwrap();
        assert!(q.page.is_none() && q.keyword.is_none());
    }

    #[test]
    fn update_dto_flattens_base() {
        let d: SeckillUpdateDTO =
            serde_json::from_str(r#"{"seckillId":7,"seckillName":"改后名称","shopId":2}"#).unwrap();
        assert_eq!(d.seckill_id, 7);
        assert_eq!(d.base.seckill_name.as_deref(), Some("改后名称"));
        assert_eq!(d.base.shop_id, Some(2));
    }

    #[test]
    fn promotion_vo_serializes_camel_case() {
        let vo = SeckillPromotionVO {
            rec_id: 1,
            seckill_id: 2,
            product_id: 3,
            sku_id: 4,
            seckill_price: Decimal::from(9),
            seckill_stock: 5,
            seckill_sales: 5,
            remain_seconds: 60,
            started: true,
            sold_percent: 50,
            ..Default::default()
        };
        let v = serde_json::to_value(&vo).unwrap();
        assert_eq!(v["seckillId"], 2);
        assert_eq!(v["remainSeconds"], 60);
        assert_eq!(v["soldPercent"], 50);
    }

    #[test]
    fn member_scope_all_or_absent_is_none() {
        // 两者皆缺、scope=0、scope 缺省 → 全部会员不落库
        assert_eq!(resolve_member_scope(None, None), None);
        let all = serde_json::json!({"scope": 0, "userIds": [], "rankIds": [], "tagIds": []});
        assert_eq!(resolve_member_scope(Some(&all), None), None);
        let no_scope = serde_json::json!({"userIds": [1]});
        assert_eq!(resolve_member_scope(Some(&no_scope), None), None);
        // 历史结构 TYPE_ALL 同样不落库
        let aud_all = serde_json::json!({"audienceType": 0});
        assert_eq!(resolve_member_scope(None, Some(&aud_all)), None);
    }

    #[test]
    fn member_scope_part_keeps_first_non_empty_group_only() {
        // 互斥归一：userIds 优先于 rankIds/tagIds
        let t = serde_json::json!({"scope": 1, "userIds": [7, 8], "rankIds": [1], "tagIds": [2]});
        let s = resolve_member_scope(Some(&t), None).unwrap();
        let v: Value = serde_json::from_str(&s).unwrap();
        assert_eq!(v["scope"], 1);
        assert_eq!(v["userIds"], serde_json::json!([7, 8]));
        assert!(v.get("rankIds").is_none() && v.get("tagIds").is_none());
        // users 明细永不落库
        let t2 = serde_json::json!({"scope": 1, "userIds": [7], "users": [{"userId": 7}]});
        let v2: Value =
            serde_json::from_str(&resolve_member_scope(Some(&t2), None).unwrap()).unwrap();
        assert!(v2.get("users").is_none());
    }

    #[test]
    fn member_scope_rank_and_tag_groups() {
        let r = serde_json::json!({"scope": 1, "rankIds": [3, 4]});
        let v: Value =
            serde_json::from_str(&resolve_member_scope(Some(&r), None).unwrap()).unwrap();
        assert_eq!(v["rankIds"], serde_json::json!([3, 4]));
        assert!(v.get("tagMatchMode").is_none());
        // tagIds 组带 tagMatchMode（缺省 0；仅 1 保留为 1）
        let g = serde_json::json!({"scope": 1, "tagIds": [5], "tagMatchMode": 1});
        let v: Value =
            serde_json::from_str(&resolve_member_scope(Some(&g), None).unwrap()).unwrap();
        assert_eq!(v["tagIds"], serde_json::json!([5]));
        assert_eq!(v["tagMatchMode"], 1);
        let g2 = serde_json::json!({"scope": 1, "tagIds": [5], "tagMatchMode": 9});
        let v2: Value =
            serde_json::from_str(&resolve_member_scope(Some(&g2), None).unwrap()).unwrap();
        assert_eq!(v2["tagMatchMode"], 0);
    }

    #[test]
    fn member_scope_audience_upgrade() {
        // 历史结构：TYPE_RANK → rankIds；TYPE_TAG → tagIds + 模式
        let rank = serde_json::json!({"audienceType": 1, "rankIds": [2]});
        let v: Value =
            serde_json::from_str(&resolve_member_scope(None, Some(&rank)).unwrap()).unwrap();
        assert_eq!(v["scope"], 1);
        assert_eq!(v["rankIds"], serde_json::json!([2]));
        let tag = serde_json::json!({"audienceType": 2, "tagIds": [9], "tagMatchMode": 1});
        let v: Value =
            serde_json::from_str(&resolve_member_scope(None, Some(&tag)).unwrap()).unwrap();
        assert_eq!(v["tagIds"], serde_json::json!([9]));
        assert_eq!(v["tagMatchMode"], 1);
        // sendUserTarget 在场时优先，memberAudience 被忽略
        let t = serde_json::json!({"scope": 0});
        assert_eq!(resolve_member_scope(Some(&t), Some(&rank)), None);
    }

    #[test]
    fn create_dto_parses_member_fields() {
        let d: SeckillCreateDTO = serde_json::from_str(
            r#"{"seckillName":"x","memberRange":1,"memberMarks":[1,2],
                "sendUserTarget":{"scope":1,"tagIds":[3]}}"#,
        )
        .unwrap();
        assert_eq!(d.member_range, Some(1));
        assert_eq!(d.member_marks.as_deref(), Some([1, 2].as_slice()));
        assert!(d.send_user_target.is_some());
        // 旧调用方不传会员字段时全为 None，不影响既有解析
        let d2: SeckillCreateDTO = serde_json::from_str(r#"{"seckillName":"y"}"#).unwrap();
        assert!(d2.member_range.is_none() && d2.member_marks.is_none());
        assert!(d2.send_user_target.is_none() && d2.member_audience.is_none());
    }
}
