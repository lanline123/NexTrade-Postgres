//! AI 用量记录（共享层，2026-11-21 自 admin crate 下沉并扩维）。
//!
//! `ai_usage` 一张台账两边记账：运营助手 scene=`admin`（shop_id/admin_id 维度），
//! 买家导购 scene=`guide`（user_id 维度、shop_id=0、admin_id=0）。扩维列
//! `user_id / scene` 由迁移 `rust/deploy/pg/2026-11-21_ai_guide.sql` 添加
//! （存量行默认 admin/0，下沉前的列表查询不读新列、零改动兼容）。
//!
//! 计费口径不变：只记不扣（AI 助手 P1 口径），`credits_charged` 由调用方给值。

/// 场景：运营助手（admin 二进制）。
pub const SCENE_ADMIN: &str = "admin";
/// 场景：买家导购（public 二进制）。
pub const SCENE_GUIDE: &str = "guide";

/// 用途 kind：1 对话 / 2 图片 / 3 翻译 / 4 消除（与表注释一致）。
pub const KIND_CHAT: i32 = 1;

/// 一条用量记录（全维度）。
#[derive(Debug, Clone)]
pub struct UsageRecord {
    /// 租户店铺：0 = 平台；导购恒 0。
    pub shop_id: i32,
    /// 管理员 id；导购恒 0。
    pub admin_id: i32,
    /// 买家 user id；运营助手恒 0。
    pub user_id: i32,
    /// `SCENE_ADMIN` / `SCENE_GUIDE`。
    pub scene: &'static str,
    /// 会话 id（各自场景的会话表内 id）。
    pub conv_id: i32,
    pub channel_id: i32,
    pub model_code: String,
    /// 见 `KIND_*`。
    pub kind: i32,
    pub prompt_tokens: i32,
    pub completion_tokens: i32,
    pub image_count: i32,
    pub credits_charged: i64,
    /// 1 = 第三方自有 key 不扣分。
    pub self_key: i32,
    /// 1 成功 / 2 失败 / 3 部分（流中断等）。
    pub status: i32,
    pub error: String,
}

impl UsageRecord {
    /// 运营助手对话用量（与下沉前 `record_usage` 的写入口径逐项一致：
    /// kind=1、image_count=0、credits=0、self_key=1、scene=admin、user_id=0）。
    #[must_use]
    pub fn admin_chat(
        shop_id: i32,
        admin_id: i32,
        conv_id: i32,
        channel_id: i32,
        model_code: &str,
        prompt_tokens: i32,
        completion_tokens: i32,
        status: i32,
        error: &str,
    ) -> Self {
        Self {
            shop_id,
            admin_id,
            user_id: 0,
            scene: SCENE_ADMIN,
            conv_id,
            channel_id,
            model_code: model_code.to_string(),
            kind: KIND_CHAT,
            prompt_tokens,
            completion_tokens,
            image_count: 0,
            credits_charged: 0,
            self_key: 1,
            status,
            error: error.to_string(),
        }
    }

    /// 买家导购对话用量（平台自担成本：credits 恒 0、self_key 恒 1）。
    #[must_use]
    pub fn guide_chat(
        user_id: i32,
        conv_id: i32,
        channel_id: i32,
        model_code: &str,
        prompt_tokens: i32,
        completion_tokens: i32,
        status: i32,
        error: &str,
    ) -> Self {
        Self {
            shop_id: 0,
            admin_id: 0,
            user_id,
            scene: SCENE_GUIDE,
            conv_id,
            channel_id,
            model_code: model_code.to_string(),
            kind: KIND_CHAT,
            prompt_tokens,
            completion_tokens,
            image_count: 0,
            credits_charged: 0,
            self_key: 1,
            status,
            error: error.to_string(),
        }
    }
}

/// 写入一条用量（失败吞错：记账失败不阻断对话，与下沉前一致）。
pub async fn record_usage(pool: &sqlx::PgPool, rec: &UsageRecord) {
    let _ = sqlx::query(
        "INSERT INTO ai_usage (shop_id, admin_id, user_id, scene, conv_id, channel_id, model_code, kind,
            prompt_tokens, completion_tokens, image_count, credits_charged, self_key, status, error, create_time)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16)",
    )
    .bind(rec.shop_id)
    .bind(rec.admin_id)
    .bind(rec.user_id)
    .bind(rec.scene)
    .bind(rec.conv_id)
    .bind(rec.channel_id)
    .bind(rec.model_code)
    .bind(rec.kind)
    .bind(rec.prompt_tokens)
    .bind(rec.completion_tokens)
    .bind(rec.image_count)
    .bind(rec.credits_charged)
    .bind(rec.self_key)
    .bind(rec.status)
    .bind(rec.error.chars().take(500).collect::<String>())
    .bind(super::now_secs())
    .execute(pool)
    .await;
}

/// 某场景在 `[start, end)` epoch 秒窗内的 token 总量（导购预算熔断用）。
pub async fn sum_tokens(
    pool: &sqlx::PgPool,
    scene: &str,
    user_id: Option<i32>,
    start: i64,
    end: i64,
) -> i64 {
    let row: (i64,) = sqlx::query_as(
        "SELECT COALESCE(SUM(prompt_tokens + completion_tokens), 0)::bigint FROM ai_usage
         WHERE scene = $1 AND ($2::int IS NULL OR user_id = $2)
           AND create_time >= $3 AND create_time < $4",
    )
    .bind(scene)
    .bind(user_id)
    .bind(start)
    .bind(end)
    .fetch_one(pool)
    .await
    .unwrap_or((0,));
    row.0
}
