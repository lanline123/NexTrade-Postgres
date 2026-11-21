//! 平台联邦仓储（hub::federation）。
//!
//! 表（`rust/deploy/pg/2026-11-20_hub_federation.sql`）：
//! - `hub_credential` + 店铺/商品/分类/覆盖 4 子表
//! - `hub_link`（类型③连接，兼实例注册表）
//! - `hub_pairing_code`、`hub_call_log`、`hub_product_map`
//! - 二期（迁移 028）：`hub_member_link` / `hub_member_map` /
//!   `hub_member_conflict`（会员同步）、`hub_order_summary`（订单上行汇总）
//!
//! SQL 一律运行时 `sqlx::query` / `query_as`，`$n` 占位。

use sqlx::{PgPool, Postgres, QueryBuilder};

use nextrade_core::models::hub::federation::{
    HubAggregationListQuery, HubCallLogListQuery, HubConfigAck, HubConfigAckListQuery,
    HubConfigApply, HubConfigApplyListQuery, HubConfigListQuery, HubConfigPackage, HubCredential,
    HubCredentialListQuery, HubLink, HubLinkListQuery, HubMemberConflict,
    HubMemberConflictListQuery, HubMemberLink, HubMemberListQuery, HubMemberMap, HubOrderSummary,
    HubOrderSummaryListQuery, HubPairingCode, HubPriceOverride, HubProductMap, HubSsoListQuery,
    HubSsoRequest, HubSsoState, LocalMemberRow, LocalOrderRow, OrderReportItem,
};
use nextrade_core::{Error, Result};

/// 平台联邦仓储。
#[derive(Clone)]
pub struct HubFederationRepository {
    pool: PgPool,
}

impl HubFederationRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    // ———————————————————— 凭证 ————————————————————

    pub async fn list_credentials(
        &self,
        q: &HubCredentialListQuery,
    ) -> Result<(Vec<HubCredential>, i64)> {
        let page = q.page.unwrap_or(1).max(1);
        let size = q.page_size.unwrap_or(20).clamp(1, 100);
        let offset = ((page - 1) * size) as i64;

        let total: (i64,) = cred_count_query(q)
            .build_query_as()
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;

        let rows = cred_page_query(q, size as i64, offset)
            .build_query_as::<HubCredential>()
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok((rows, total.0.max(0)))
    }

    pub async fn find_credential_by_id(
        &self,
        credential_id: i32,
    ) -> Result<Option<HubCredential>> {
        sqlx::query_as::<_, HubCredential>(
            "SELECT * FROM hub_credential WHERE credential_id = $1",
        )
        .bind(credential_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 鉴权专用查找：按 access_key 取**入站**行（我签发、只存哈希的那一行）。
    /// 同一把钥在持有方另有一行出站孪生行（同 access_key、direction=2），
    /// 回环同库时两行共存——鉴权必须定向取入站行，不许按钥裸查后靠行序碰运气；
    /// (access_key, direction) 唯一索引保证至多一行。
    pub async fn find_inbound_credential_by_access_key(
        &self,
        access_key: &str,
    ) -> Result<Option<HubCredential>> {
        sqlx::query_as::<_, HubCredential>(
            "SELECT * FROM hub_credential WHERE access_key = $1 AND direction = 1",
        )
        .bind(access_key)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 按（钥、方向、连接）精确查一行：配对落钥前的复用判定
    /// （同一连接同一角色方向至多一行，见 (access_key, direction) 唯一口径）。
    pub async fn find_credential_by_key_dir_link(
        &self,
        access_key: &str,
        direction: i32,
        link_id: i32,
    ) -> Result<Option<HubCredential>> {
        sqlx::query_as::<_, HubCredential>(
            "SELECT * FROM hub_credential WHERE access_key = $1 AND direction = $2 AND link_id = $3",
        )
        .bind(access_key)
        .bind(direction)
        .bind(link_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 查某连接在某方向上已落库的凭证（配对流程每方向至多一把角色钥）：
    /// poll 重入时据此复用已领取的上行钥，不再重复向中枢领取。
    pub async fn find_link_credential(
        &self,
        link_id: i32,
        direction: i32,
    ) -> Result<Option<HubCredential>> {
        sqlx::query_as::<_, HubCredential>(
            "SELECT * FROM hub_credential WHERE link_id = $1 AND direction = $2 ORDER BY credential_id ASC LIMIT 1",
        )
        .bind(link_id)
        .bind(direction)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn insert_credential(
        &self,
        conn_type: i32,
        link_id: i32,
        direction: i32,
        peer_base_url: &str,
        cred_name: &str,
        access_key: &str,
        secret_hash: &str,
        secret_enc: &str,
        scopes: i64,
        scope_all_shops: i32,
        scope_mode: i32,
        include_inquiry: i32,
        price_rule_type: &str,
        price_rule_value: rust_decimal::Decimal,
        expire_time: i32,
        rate_limit_per_min: i32,
        daily_quota: i64,
        ip_whitelist: &str,
        now: i32,
    ) -> Result<i32> {
        let row: (i32,) = sqlx::query_as(
            "INSERT INTO hub_credential (conn_type, link_id, direction, peer_base_url, cred_name, access_key, secret_hash, secret_enc, scopes, scope_all_shops, scope_mode, include_inquiry, price_rule_type, price_rule_value, status, expire_time, rate_limit_per_min, daily_quota, ip_whitelist, last_used_time, call_count, add_time, update_time)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, 1, $15, $16, $17, $18, 0, 0, $19, $19) RETURNING credential_id",
        )
        .bind(conn_type)
        .bind(link_id)
        .bind(direction)
        .bind(peer_base_url)
        .bind(cred_name)
        .bind(access_key)
        .bind(secret_hash)
        .bind(secret_enc)
        .bind(scopes)
        .bind(scope_all_shops)
        .bind(scope_mode)
        .bind(include_inquiry)
        .bind(price_rule_type)
        .bind(price_rule_value)
        .bind(expire_time)
        .bind(rate_limit_per_min)
        .bind(daily_quota)
        .bind(ip_whitelist)
        .bind(now)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.0)
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn update_credential_fields(
        &self,
        credential_id: i32,
        cred_name: &str,
        scopes: i64,
        scope_all_shops: i32,
        scope_mode: i32,
        include_inquiry: i32,
        price_rule_type: &str,
        price_rule_value: rust_decimal::Decimal,
        status: i32,
        expire_time: i32,
        rate_limit_per_min: i32,
        daily_quota: i64,
        ip_whitelist: &str,
        peer_base_url: &str,
        now: i32,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE hub_credential SET cred_name = $2, scopes = $3, scope_all_shops = $4, scope_mode = $5, include_inquiry = $6, price_rule_type = $7, price_rule_value = $8, status = $9, expire_time = $10, rate_limit_per_min = $11, daily_quota = $12, ip_whitelist = $13, peer_base_url = $14, update_time = $15 WHERE credential_id = $1",
        )
        .bind(credential_id)
        .bind(cred_name)
        .bind(scopes)
        .bind(scope_all_shops)
        .bind(scope_mode)
        .bind(include_inquiry)
        .bind(price_rule_type)
        .bind(price_rule_value)
        .bind(status)
        .bind(expire_time)
        .bind(rate_limit_per_min)
        .bind(daily_quota)
        .bind(ip_whitelist)
        .bind(peer_base_url)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    pub async fn update_credential_access_key(
        &self,
        credential_id: i32,
        access_key: &str,
        now: i32,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE hub_credential SET access_key = $2, update_time = $3 WHERE credential_id = $1",
        )
        .bind(credential_id)
        .bind(access_key)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    pub async fn update_credential_secret_hash(
        &self,
        credential_id: i32,
        secret_hash: &str,
        now: i32,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE hub_credential SET secret_hash = $2, update_time = $3 WHERE credential_id = $1",
        )
        .bind(credential_id)
        .bind(secret_hash)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    pub async fn update_credential_secret_enc(
        &self,
        credential_id: i32,
        secret_enc: &str,
        now: i32,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE hub_credential SET secret_enc = $2, update_time = $3 WHERE credential_id = $1",
        )
        .bind(credential_id)
        .bind(secret_enc)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    pub async fn update_credential_link(
        &self,
        credential_id: i32,
        link_id: i32,
        now: i32,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE hub_credential SET link_id = $2, update_time = $3 WHERE credential_id = $1",
        )
        .bind(credential_id)
        .bind(link_id)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    pub async fn delete_credential(&self, credential_id: i32) -> Result<()> {
        // hub_product_map 同在清理之列：单向接入（类型②）凭证的聚合映射只挂
        // credential_id（link_id=0），删连接的事务清理覆盖不到，删凭证时不清
        // 即成无主孤儿（与 delete_link 清映射同一口径）。
        for table in [
            "hub_credential_shop",
            "hub_credential_product",
            "hub_credential_category",
            "hub_credential_price_override",
            "hub_product_map",
        ] {
            sqlx::query(&format!(
                "DELETE FROM {table} WHERE credential_id = $1"
            ))
            .bind(credential_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        }
        sqlx::query("DELETE FROM hub_credential WHERE credential_id = $1")
            .bind(credential_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(())
    }

    pub async fn touch_credential_usage(&self, credential_id: i32, now: i32) -> Result<()> {
        sqlx::query(
            "UPDATE hub_credential SET last_used_time = $2, call_count = call_count + 1 WHERE credential_id = $1",
        )
        .bind(credential_id)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    // ———————————————————— 凭证子表 ————————————————————

    pub async fn replace_id_list(
        &self,
        table: &str,
        column: &str,
        credential_id: i32,
        ids: &[i32],
    ) -> Result<()> {
        sqlx::query(&format!("DELETE FROM {table} WHERE credential_id = $1"))
            .bind(credential_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        for id in ids {
            sqlx::query(&format!(
                "INSERT INTO {table} (credential_id, {column}) VALUES ($1, $2) ON CONFLICT DO NOTHING"
            ))
            .bind(credential_id)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        }
        Ok(())
    }

    pub async fn list_id_list(
        &self,
        table: &str,
        column: &str,
        credential_id: i32,
    ) -> Result<Vec<i32>> {
        let rows: Vec<(i32,)> = sqlx::query_as(&format!(
            "SELECT {column} FROM {table} WHERE credential_id = $1 ORDER BY {column} ASC"
        ))
        .bind(credential_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(rows.into_iter().map(|r| r.0).collect())
    }

    pub async fn list_all_overrides(
        &self,
        credential_id: i32,
    ) -> Result<Vec<HubPriceOverride>> {
        sqlx::query_as::<_, HubPriceOverride>(
            "SELECT * FROM hub_credential_price_override WHERE credential_id = $1",
        )
        .bind(credential_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    pub async fn upsert_override(
        &self,
        credential_id: i32,
        product_id: i32,
        rule_type: &str,
        rule_value: rust_decimal::Decimal,
        now: i32,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO hub_credential_price_override (credential_id, product_id, rule_type, rule_value, add_time, update_time) VALUES ($1, $2, $3, $4, $5, $5)
             ON CONFLICT (credential_id, product_id) DO UPDATE SET rule_type = EXCLUDED.rule_type, rule_value = EXCLUDED.rule_value, update_time = EXCLUDED.update_time",
        )
        .bind(credential_id)
        .bind(product_id)
        .bind(rule_type)
        .bind(rule_value)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    pub async fn delete_override(&self, credential_id: i32, product_id: i32) -> Result<()> {
        sqlx::query(
            "DELETE FROM hub_credential_price_override WHERE credential_id = $1 AND product_id = $2",
        )
        .bind(credential_id)
        .bind(product_id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    // ———————————————————— 连接 ————————————————————

    pub async fn list_links(&self, q: &HubLinkListQuery) -> Result<(Vec<HubLink>, i64)> {
        let page = q.page.unwrap_or(1).max(1);
        let size = q.page_size.unwrap_or(20).clamp(1, 100);
        let offset = ((page - 1) * size) as i64;

        let total: (i64,) = link_count_query(q)
            .build_query_as()
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;

        let rows = link_page_query(q, size as i64, offset)
            .build_query_as::<HubLink>()
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok((rows, total.0.max(0)))
    }

    pub async fn find_link_by_id(&self, link_id: i32) -> Result<Option<HubLink>> {
        sqlx::query_as::<_, HubLink>("SELECT * FROM hub_link WHERE link_id = $1")
            .bind(link_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn insert_link(
        &self,
        role: i32,
        peer_instance_uuid: &str,
        peer_name: &str,
        peer_base_url: &str,
        peer_version: &str,
        status: i32,
        claim_token_hash: &str,
        claim_token_enc: &str,
        now: i32,
    ) -> Result<i32> {
        let row: (i32,) = sqlx::query_as(
            "INSERT INTO hub_link (link_type, role, peer_link_id, peer_instance_uuid, peer_name, peer_base_url, peer_version, status, approved_modules, up_credential_id, down_credential_id, partner_user_id, settle_mode, settle_cycle, fund_channel, rate_source, rate_override, claim_token_hash, claim_token_enc, claim_expire_time, reject_reason, last_heartbeat_time, heartbeat_payload, add_time, update_time)
             VALUES (3, $1, 0, $2, $3, $4, $5, $6, 0, 0, 0, 0, '', '', '', 'level', 0, $7, $8, 0, '', 0, '', $9, $9) RETURNING link_id",
        )
        .bind(role)
        .bind(peer_instance_uuid)
        .bind(peer_name)
        .bind(peer_base_url)
        .bind(peer_version)
        .bind(status)
        .bind(claim_token_hash)
        .bind(claim_token_enc)
        .bind(now)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.0)
    }

    pub async fn update_link_credentials(
        &self,
        link_id: i32,
        up_credential_id: i32,
        down_credential_id: i32,
        now: i32,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE hub_link SET up_credential_id = $2, down_credential_id = $3, update_time = $4 WHERE link_id = $1",
        )
        .bind(link_id)
        .bind(up_credential_id)
        .bind(down_credential_id)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn approve_link(
        &self,
        link_id: i32,
        approved_modules: i64,
        partner_user_id: i32,
        settle_mode: &str,
        settle_cycle: &str,
        fund_channel: &str,
        rate_source: &str,
        rate_override: rust_decimal::Decimal,
        claim_expire_time: i32,
        now: i32,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE hub_link SET status = 2, approved_modules = $2, partner_user_id = $3, settle_mode = $4, settle_cycle = $5, fund_channel = $6, rate_source = $7, rate_override = $8, claim_expire_time = $9, reject_reason = '', update_time = $10 WHERE link_id = $1",
        )
        .bind(link_id)
        .bind(approved_modules)
        .bind(partner_user_id)
        .bind(settle_mode)
        .bind(settle_cycle)
        .bind(fund_channel)
        .bind(rate_source)
        .bind(rate_override)
        .bind(claim_expire_time)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    pub async fn update_link_status(
        &self,
        link_id: i32,
        status: i32,
        reject_reason: &str,
        now: i32,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE hub_link SET status = $2, reject_reason = $3, update_time = $4 WHERE link_id = $1",
        )
        .bind(link_id)
        .bind(status)
        .bind(reject_reason)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    pub async fn clear_link_claim_token(&self, link_id: i32, now: i32) -> Result<()> {
        sqlx::query(
            "UPDATE hub_link SET claim_token_hash = '', claim_token_enc = '', claim_expire_time = 0, update_time = $2 WHERE link_id = $1",
        )
        .bind(link_id)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    pub async fn update_link_peer_url(
        &self,
        link_id: i32,
        peer_base_url: &str,
        now: i32,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE hub_link SET peer_base_url = $2, update_time = $3 WHERE link_id = $1",
        )
        .bind(link_id)
        .bind(peer_base_url)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    pub async fn update_link_heartbeat(
        &self,
        link_id: i32,
        payload: &str,
        now: i32,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE hub_link SET last_heartbeat_time = $2, heartbeat_payload = $3, update_time = $2 WHERE link_id = $1",
        )
        .bind(link_id)
        .bind(now)
        .bind(payload)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    /// 删除连接本体：同一事务内先清该连接的聚合映射（hub_product_map），
    /// 再删 hub_link 行。映射是连接聚合行为的从属行，连接删除后残留即无主
    /// 孤儿（2026-11-21 验收观察项：API 删连接后残 63 行映射靠人工清扫）。
    /// 调用日志（hub_call_log）属审计流水，不在此列、保留。
    pub async fn delete_link(&self, link_id: i32) -> Result<()> {
        let mut tx = self.pool.begin().await.map_err(map_err)?;
        sqlx::query("DELETE FROM hub_product_map WHERE link_id = $1")
            .bind(link_id)
            .execute(&mut *tx)
            .await
            .map_err(map_err)?;
        sqlx::query("DELETE FROM hub_link WHERE link_id = $1")
            .bind(link_id)
            .execute(&mut *tx)
            .await
            .map_err(map_err)?;
        tx.commit().await.map_err(map_err)?;
        Ok(())
    }

    /// 成员侧回填：对端（中枢）库中的连接 id。
    pub async fn update_link_peer_link_id(
        &self,
        link_id: i32,
        peer_link_id: i32,
        now: i32,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE hub_link SET peer_link_id = $2, update_time = $3 WHERE link_id = $1",
        )
        .bind(link_id)
        .bind(peer_link_id)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    /// 成员侧领取激活：置已激活 + 落中枢批准的模块上限。
    /// 只落批准模块、不动状态：成员领取上行钥**之前**先行落库，
    /// 使领取后回填失败时，poll 的复用路径能从本地行读到批准模块补完激活。
    pub async fn update_link_approved_modules(
        &self,
        link_id: i32,
        approved_modules: i64,
        now: i32,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE hub_link SET approved_modules = $2, update_time = $3 WHERE link_id = $1",
        )
        .bind(link_id)
        .bind(approved_modules)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    pub async fn update_link_activation(
        &self,
        link_id: i32,
        approved_modules: i64,
        now: i32,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE hub_link SET status = 2, approved_modules = $2, update_time = $3 WHERE link_id = $1",
        )
        .bind(link_id)
        .bind(approved_modules)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    /// 配对码近况列表（管理端；只回行结构，哈希不出服务层）。
    pub async fn list_pairing_codes(&self) -> Result<Vec<HubPairingCode>> {
        sqlx::query_as::<_, HubPairingCode>(
            "SELECT * FROM hub_pairing_code ORDER BY id DESC LIMIT 100",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 作废一张未用配对码。
    pub async fn void_pairing_code(&self, id: i32) -> Result<()> {
        sqlx::query("UPDATE hub_pairing_code SET status = 2 WHERE id = $1 AND status = 0")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(())
    }

    /// 按业务键查一条聚合映射（同步增量判定用）。
    pub async fn find_map_by_key(
        &self,
        link_id: i32,
        credential_id: i32,
        remote_product_id: i32,
    ) -> Result<Option<HubProductMap>> {
        sqlx::query_as::<_, HubProductMap>(
            "SELECT * FROM hub_product_map WHERE link_id = $1 AND credential_id = $2 AND remote_product_id = $3",
        )
        .bind(link_id)
        .bind(credential_id)
        .bind(remote_product_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    // ———————————————————— 配对码 ————————————————————

    pub async fn insert_pairing_code(
        &self,
        code_hash: &str,
        expect_name: &str,
        expire_time: i32,
        now: i32,
    ) -> Result<i32> {
        let row: (i32,) = sqlx::query_as(
            "INSERT INTO hub_pairing_code (code_hash, expect_name, status, fail_count, expire_time, used_link_id, add_time) VALUES ($1, $2, 0, 0, $3, 0, $4) RETURNING id",
        )
        .bind(code_hash)
        .bind(expect_name)
        .bind(expire_time)
        .bind(now)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.0)
    }

    pub async fn find_pairing_by_hash(
        &self,
        code_hash: &str,
    ) -> Result<Option<HubPairingCode>> {
        sqlx::query_as::<_, HubPairingCode>(
            "SELECT * FROM hub_pairing_code WHERE code_hash = $1 ORDER BY id DESC LIMIT 1",
        )
        .bind(code_hash)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    pub async fn mark_pairing_used(&self, id: i32, link_id: i32) -> Result<()> {
        sqlx::query("UPDATE hub_pairing_code SET status = 1, used_link_id = $2 WHERE id = $1")
            .bind(id)
            .bind(link_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(())
    }

    pub async fn bump_pairing_fail(&self, id: i32, void: bool) -> Result<()> {
        if void {
            sqlx::query(
                "UPDATE hub_pairing_code SET fail_count = fail_count + 1, status = 2 WHERE id = $1",
            )
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        } else {
            sqlx::query("UPDATE hub_pairing_code SET fail_count = fail_count + 1 WHERE id = $1")
                .bind(id)
                .execute(&self.pool)
                .await
                .map_err(map_err)?;
        }
        Ok(())
    }

    // ———————————————————— 调用日志 ————————————————————

    #[allow(clippy::too_many_arguments)]
    pub async fn insert_call_log(
        &self,
        link_id: i32,
        credential_id: i32,
        access_key: &str,
        endpoint: &str,
        scope: &str,
        direction: i32,
        http_status: i32,
        result_count: i32,
        client_ip: &str,
        duration_ms: i32,
        now: i32,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO hub_call_log (link_id, credential_id, access_key, endpoint, scope, direction, http_status, result_count, client_ip, duration_ms, add_time) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
        )
        .bind(link_id)
        .bind(credential_id)
        .bind(access_key)
        .bind(endpoint)
        .bind(scope)
        .bind(direction)
        .bind(http_status)
        .bind(result_count)
        .bind(client_ip)
        .bind(duration_ms)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    pub async fn list_call_logs(
        &self,
        q: &HubCallLogListQuery,
    ) -> Result<(Vec<nextrade_core::models::hub::federation::HubCallLogRow>, i64)> {
        let page = q.page.unwrap_or(1).max(1);
        let size = q.page_size.unwrap_or(20).clamp(1, 100);
        let offset = ((page - 1) * size) as i64;

        let total: (i64,) = log_count_query(q)
            .build_query_as()
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;

        let rows = log_page_query(q, size as i64, offset)
            .build_query_as::<nextrade_core::models::hub::federation::HubCallLogRow>()
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok((rows, total.0.max(0)))
    }

    /// 今日某凭证的成功调用数（日配额校验用）。
    pub async fn count_calls_today(&self, credential_id: i32, day_start: i32) -> Result<i64> {
        let row: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM hub_call_log WHERE credential_id = $1 AND add_time >= $2",
        )
        .bind(credential_id)
        .bind(day_start)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.0.max(0))
    }

    // ———————————————————— 商品聚合映射 ————————————————————

    #[allow(clippy::too_many_arguments)]
    pub async fn upsert_product_map(
        &self,
        link_id: i32,
        credential_id: i32,
        remote_instance_uuid: &str,
        remote_shop_id: i32,
        remote_shop_name: &str,
        remote_product_id: i32,
        product_name: &str,
        pic_url: &str,
        price: rust_decimal::Decimal,
        market_price: rust_decimal::Decimal,
        stock: i32,
        content_hash: &str,
        snapshot: &str,
        now: i32,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO hub_product_map (link_id, credential_id, remote_instance_uuid, remote_shop_id, remote_shop_name, remote_product_id, hub_product_id, product_name, pic_url, price, market_price, stock, content_hash, snapshot, sync_state, last_sync_time, add_time, update_time)
             VALUES ($1, $2, $3, $4, $5, $6, 0, $7, $8, $9, $10, $11, $12, $13, 1, $14, $14, $14)
             ON CONFLICT (link_id, credential_id, remote_product_id) DO UPDATE SET remote_shop_id = EXCLUDED.remote_shop_id, remote_shop_name = EXCLUDED.remote_shop_name, product_name = EXCLUDED.product_name, pic_url = EXCLUDED.pic_url, price = EXCLUDED.price, market_price = EXCLUDED.market_price, stock = EXCLUDED.stock, content_hash = EXCLUDED.content_hash, snapshot = EXCLUDED.snapshot, sync_state = 1, last_sync_time = EXCLUDED.last_sync_time, update_time = EXCLUDED.update_time",
        )
        .bind(link_id)
        .bind(credential_id)
        .bind(remote_instance_uuid)
        .bind(remote_shop_id)
        .bind(remote_shop_name)
        .bind(remote_product_id)
        .bind(product_name)
        .bind(pic_url)
        .bind(price)
        .bind(market_price)
        .bind(stock)
        .bind(content_hash)
        .bind(snapshot)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    pub async fn mark_map_missing(
        &self,
        link_id: i32,
        credential_id: i32,
        seen_remote_ids: &[i32],
        now: i32,
    ) -> Result<()> {
        if seen_remote_ids.is_empty() {
            sqlx::query(
                "UPDATE hub_product_map SET sync_state = 3, update_time = $3 WHERE link_id = $1 AND credential_id = $2 AND sync_state = 1",
            )
            .bind(link_id)
            .bind(credential_id)
            .bind(now)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
            return Ok(());
        }
        let mut qb: QueryBuilder<'_, Postgres> = QueryBuilder::new(
            "UPDATE hub_product_map SET sync_state = 3, update_time = ",
        );
        qb.push_bind(now);
        qb.push(" WHERE link_id = ");
        qb.push_bind(link_id);
        qb.push(" AND credential_id = ");
        qb.push_bind(credential_id);
        qb.push(" AND sync_state = 1 AND remote_product_id NOT IN (");
        let mut sep = qb.separated(", ");
        for id in seen_remote_ids {
            sep.push_bind(*id);
        }
        sep.push_unseparated(")");
        drop(sep);
        qb.build()
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(())
    }

    pub async fn set_map_state(&self, map_id: i32, sync_state: i32, now: i32) -> Result<()> {
        sqlx::query(
            "UPDATE hub_product_map SET sync_state = $2, update_time = $3 WHERE map_id = $1",
        )
        .bind(map_id)
        .bind(sync_state)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    pub async fn list_product_maps(
        &self,
        q: &HubAggregationListQuery,
    ) -> Result<(Vec<HubProductMap>, i64)> {
        let page = q.page.unwrap_or(1).max(1);
        let size = q.page_size.unwrap_or(20).clamp(1, 100);
        let offset = ((page - 1) * size) as i64;

        let total: (i64,) = map_count_query(q)
            .build_query_as()
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;

        let rows = map_page_query(q, size as i64, offset)
            .build_query_as::<HubProductMap>()
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok((rows, total.0.max(0)))
    }

    pub async fn count_maps(&self) -> Result<i64> {
        let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM hub_product_map")
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(row.0.max(0))
    }

    pub async fn count_links_by_status(&self, status: i32) -> Result<i64> {
        let row: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM hub_link WHERE status = $1")
                .bind(status)
                .fetch_one(&self.pool)
                .await
                .map_err(map_err)?;
        Ok(row.0.max(0))
    }

    pub async fn count_credentials(&self) -> Result<i64> {
        let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM hub_credential")
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(row.0.max(0))
    }

    // ———————————————————— 会员同步（迁移 028，设计 §6） ————————————————————

    /// 中枢侧：按（连接, 远端会员）查映射行。
    pub async fn find_member_link_by_remote(
        &self,
        link_id: i32,
        remote_member_id: i32,
    ) -> Result<Option<HubMemberLink>> {
        sqlx::query_as::<_, HubMemberLink>(
            "SELECT * FROM hub_member_link WHERE link_id = $1 AND remote_member_id = $2",
        )
        .bind(link_id)
        .bind(remote_member_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 中枢侧：同手机号的全部已关联映射（匹配裁决入参）。
    pub async fn list_member_links_by_mobile_hash(
        &self,
        mobile_hash: &str,
    ) -> Result<Vec<HubMemberLink>> {
        sqlx::query_as::<_, HubMemberLink>(
            "SELECT * FROM hub_member_link WHERE mobile_hash = $1 AND status = 1 ORDER BY id ASC",
        )
        .bind(mobile_hash)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 中枢侧：按全局 ID 查任一行（锚点核对/冲突快照取实名用）。
    pub async fn find_member_link_by_global(
        &self,
        global_member_id: &str,
    ) -> Result<Option<HubMemberLink>> {
        sqlx::query_as::<_, HubMemberLink>(
            "SELECT * FROM hub_member_link WHERE global_member_id = $1 ORDER BY id ASC LIMIT 1",
        )
        .bind(global_member_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 中枢侧：按（连接, 全局 ID）查该连接名下的映射行（锚点核对用）。
    pub async fn find_member_link_by_link_global(
        &self,
        link_id: i32,
        global_member_id: &str,
    ) -> Result<Option<HubMemberLink>> {
        sqlx::query_as::<_, HubMemberLink>(
            "SELECT * FROM hub_member_link WHERE link_id = $1 AND global_member_id = $2 LIMIT 1",
        )
        .bind(link_id)
        .bind(global_member_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn insert_member_link(
        &self,
        global_member_id: &str,
        link_id: i32,
        remote_member_id: i32,
        mobile_hash: &str,
        mobile_enc: &str,
        nickname: &str,
        avatar: &str,
        birthday: &str,
        email: &str,
        real_name: &str,
        profile_updated_at: i32,
        now: i32,
    ) -> Result<i32> {
        let row: (i32,) = sqlx::query_as(
            "INSERT INTO hub_member_link (global_member_id, link_id, remote_member_id, mobile_hash, mobile_enc, nickname, avatar, birthday, email, real_name, profile_updated_at, status, add_time, update_time)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, 1, $12, $12) RETURNING id",
        )
        .bind(global_member_id)
        .bind(link_id)
        .bind(remote_member_id)
        .bind(mobile_hash)
        .bind(mobile_enc)
        .bind(nickname)
        .bind(avatar)
        .bind(birthday)
        .bind(email)
        .bind(real_name)
        .bind(profile_updated_at)
        .bind(now)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.0)
    }

    /// 中枢侧：刷新映射快照（调用方已做后写优先比较）+ 置已关联（解绑后重报即复活）。
    #[allow(clippy::too_many_arguments)]
    pub async fn update_member_link_snapshot(
        &self,
        id: i32,
        mobile_hash: &str,
        mobile_enc: &str,
        nickname: &str,
        avatar: &str,
        birthday: &str,
        email: &str,
        real_name: &str,
        profile_updated_at: i32,
        now: i32,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE hub_member_link SET mobile_hash = $2, mobile_enc = $3, nickname = $4, avatar = $5, birthday = $6, email = $7, real_name = $8, profile_updated_at = $9, status = 1, update_time = $10 WHERE id = $1",
        )
        .bind(id)
        .bind(mobile_hash)
        .bind(mobile_enc)
        .bind(nickname)
        .bind(avatar)
        .bind(birthday)
        .bind(email)
        .bind(real_name)
        .bind(profile_updated_at)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    pub async fn set_member_link_status(
        &self,
        link_id: i32,
        remote_member_id: i32,
        status: i32,
        now: i32,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE hub_member_link SET status = $3, update_time = $4 WHERE link_id = $1 AND remote_member_id = $2",
        )
        .bind(link_id)
        .bind(remote_member_id)
        .bind(status)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    pub async fn list_member_links(
        &self,
        q: &HubMemberListQuery,
    ) -> Result<(Vec<HubMemberLink>, i64)> {
        let page = q.page.unwrap_or(1).max(1);
        let size = q.page_size.unwrap_or(20).clamp(1, 100);
        let offset = ((page - 1) * size) as i64;

        let total: (i64,) = member_count_query(q)
            .build_query_as()
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;

        let rows = member_page_query(q, size as i64, offset)
            .build_query_as::<HubMemberLink>()
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok((rows, total.0.max(0)))
    }

    pub async fn count_member_links(&self) -> Result<i64> {
        let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM hub_member_link WHERE status = 1")
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(row.0.max(0))
    }

    /// 冲突入队（同连接+远端会员+类型已有待处理行则跳过，防重复上报刷屏队列）。
    #[allow(clippy::too_many_arguments)]
    pub async fn insert_member_conflict(
        &self,
        link_id: i32,
        remote_member_id: i32,
        mobile_hash: &str,
        mobile_enc: &str,
        nickname: &str,
        avatar: &str,
        birthday: &str,
        email: &str,
        real_name: &str,
        profile_updated_at: i32,
        candidate_global_member_id: &str,
        conflict_type: &str,
        now: i32,
    ) -> Result<()> {
        let existing: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM hub_member_conflict WHERE link_id = $1 AND remote_member_id = $2 AND conflict_type = $3 AND status = 0",
        )
        .bind(link_id)
        .bind(remote_member_id)
        .bind(conflict_type)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        if existing.0 > 0 {
            return Ok(());
        }
        sqlx::query(
            "INSERT INTO hub_member_conflict (link_id, remote_member_id, mobile_hash, mobile_enc, nickname, avatar, birthday, email, real_name, profile_updated_at, candidate_global_member_id, conflict_type, status, add_time, update_time)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, 0, $13, $13)",
        )
        .bind(link_id)
        .bind(remote_member_id)
        .bind(mobile_hash)
        .bind(mobile_enc)
        .bind(nickname)
        .bind(avatar)
        .bind(birthday)
        .bind(email)
        .bind(real_name)
        .bind(profile_updated_at)
        .bind(candidate_global_member_id)
        .bind(conflict_type)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    pub async fn find_member_conflict(&self, id: i32) -> Result<Option<HubMemberConflict>> {
        sqlx::query_as::<_, HubMemberConflict>(
            "SELECT * FROM hub_member_conflict WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    pub async fn update_member_conflict_status(
        &self,
        id: i32,
        status: i32,
        now: i32,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE hub_member_conflict SET status = $2, update_time = $3 WHERE id = $1 AND status = 0",
        )
        .bind(id)
        .bind(status)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    pub async fn list_member_conflicts(
        &self,
        q: &HubMemberConflictListQuery,
    ) -> Result<(Vec<HubMemberConflict>, i64)> {
        let page = q.page.unwrap_or(1).max(1);
        let size = q.page_size.unwrap_or(20).clamp(1, 100);
        let offset = ((page - 1) * size) as i64;

        let total: (i64,) = conflict_count_query(q)
            .build_query_as()
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;

        let rows = conflict_page_query(q, size as i64, offset)
            .build_query_as::<HubMemberConflict>()
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok((rows, total.0.max(0)))
    }

    pub async fn count_pending_conflicts(&self) -> Result<i64> {
        let row: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM hub_member_conflict WHERE status = 0")
                .fetch_one(&self.pool)
                .await
                .map_err(map_err)?;
        Ok(row.0.max(0))
    }

    /// 成员侧：落/刷新本地映射（同步回执与 query 下钻共用）。
    pub async fn upsert_member_map(
        &self,
        link_id: i32,
        local_user_id: i32,
        global_member_id: &str,
        profile_updated_at: i32,
        now: i32,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO hub_member_map (link_id, local_user_id, global_member_id, profile_updated_at, add_time, update_time)
             VALUES ($1, $2, $3, $4, $5, $5)
             ON CONFLICT (link_id, local_user_id) DO UPDATE SET global_member_id = EXCLUDED.global_member_id, profile_updated_at = EXCLUDED.profile_updated_at, update_time = EXCLUDED.update_time",
        )
        .bind(link_id)
        .bind(local_user_id)
        .bind(global_member_id)
        .bind(profile_updated_at)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    pub async fn find_member_map_by_global(
        &self,
        link_id: i32,
        global_member_id: &str,
    ) -> Result<Option<HubMemberMap>> {
        sqlx::query_as::<_, HubMemberMap>(
            "SELECT * FROM hub_member_map WHERE link_id = $1 AND global_member_id = $2 LIMIT 1",
        )
        .bind(link_id)
        .bind(global_member_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    pub async fn find_member_map_by_user(
        &self,
        link_id: i32,
        local_user_id: i32,
    ) -> Result<Option<HubMemberMap>> {
        sqlx::query_as::<_, HubMemberMap>(
            "SELECT * FROM hub_member_map WHERE link_id = $1 AND local_user_id = $2",
        )
        .bind(link_id)
        .bind(local_user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 成员侧发送端：按 user_id 游标增量扫本地会员（仅正式会员 user_type=0）。
    pub async fn list_local_members_after(
        &self,
        cursor: i32,
        limit: i64,
    ) -> Result<Vec<LocalMemberRow>> {
        sqlx::query_as::<_, LocalMemberRow>(
            "SELECT user_id, mobile, mobile_validated, email, nickname, avatar, birthday, status, reg_time, last_login FROM \"user\" WHERE user_id > $1 AND user_type = 0 ORDER BY user_id ASC LIMIT $2",
        )
        .bind(cursor)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    pub async fn find_local_member_by_id(
        &self,
        user_id: i32,
    ) -> Result<Option<LocalMemberRow>> {
        sqlx::query_as::<_, LocalMemberRow>(
            "SELECT user_id, mobile, mobile_validated, email, nickname, avatar, birthday, status, reg_time, last_login FROM \"user\" WHERE user_id = $1",
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    pub async fn find_local_member_by_mobile(
        &self,
        mobile: &str,
    ) -> Result<Option<LocalMemberRow>> {
        sqlx::query_as::<_, LocalMemberRow>(
            "SELECT user_id, mobile, mobile_validated, email, nickname, avatar, birthday, status, reg_time, last_login FROM \"user\" WHERE mobile = $1 AND user_type = 0 LIMIT 1",
        )
        .bind(mobile)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 成员侧同步游标回写（会员/订单分开推进）。
    pub async fn update_link_member_cursor(
        &self,
        link_id: i32,
        member_cursor: i32,
        now: i32,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE hub_link SET member_sync_cursor = $2, update_time = $3 WHERE link_id = $1",
        )
        .bind(link_id)
        .bind(member_cursor)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    pub async fn update_link_order_cursor(
        &self,
        link_id: i32,
        order_cursor: i32,
        order_time: i32,
        now: i32,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE hub_link SET order_sync_cursor = $2, order_sync_time = $3, update_time = $4 WHERE link_id = $1",
        )
        .bind(link_id)
        .bind(order_cursor)
        .bind(order_time)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    // ———————————————————— 订单上行（迁移 028，设计 §7.1） ————————————————————

    /// 中枢侧：按（连接, 远端订单号）查既有汇总（版本比较用）。
    pub async fn find_order_summary(
        &self,
        link_id: i32,
        remote_order_sn: &str,
    ) -> Result<Option<HubOrderSummary>> {
        sqlx::query_as::<_, HubOrderSummary>(
            "SELECT * FROM hub_order_summary WHERE link_id = $1 AND remote_order_sn = $2",
        )
        .bind(link_id)
        .bind(remote_order_sn)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 中枢侧：订单汇总 upsert（唯一键冲突更新；版本守卫在服务层先行比较）。
    pub async fn upsert_order_summary(
        &self,
        link_id: i32,
        remote_instance_uuid: &str,
        item: &OrderReportItem,
        now: i32,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO hub_order_summary (link_id, remote_instance_uuid, remote_order_sn, remote_shop_id, remote_shop_name, remote_user_id, global_member_id, order_amount, paid_amount, order_status, pay_status, item_count, order_time, pay_time, status_version, add_time, update_time)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $16)
             ON CONFLICT (link_id, remote_order_sn) DO UPDATE SET remote_instance_uuid = EXCLUDED.remote_instance_uuid, remote_shop_id = EXCLUDED.remote_shop_id, remote_shop_name = EXCLUDED.remote_shop_name, remote_user_id = EXCLUDED.remote_user_id, global_member_id = EXCLUDED.global_member_id, order_amount = EXCLUDED.order_amount, paid_amount = EXCLUDED.paid_amount, order_status = EXCLUDED.order_status, pay_status = EXCLUDED.pay_status, item_count = EXCLUDED.item_count, order_time = EXCLUDED.order_time, pay_time = EXCLUDED.pay_time, status_version = EXCLUDED.status_version, update_time = EXCLUDED.update_time",
        )
        .bind(link_id)
        .bind(remote_instance_uuid)
        .bind(&item.remote_order_sn)
        .bind(item.remote_shop_id)
        .bind(&item.remote_shop_name)
        .bind(item.remote_user_id)
        .bind(&item.global_member_id)
        .bind(item.order_amount)
        .bind(item.paid_amount)
        .bind(item.order_status)
        .bind(item.pay_status)
        .bind(item.item_count)
        .bind(item.order_time)
        .bind(item.pay_time)
        .bind(item.status_version)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    pub async fn list_order_summaries(
        &self,
        q: &HubOrderSummaryListQuery,
    ) -> Result<(Vec<HubOrderSummary>, i64)> {
        let page = q.page.unwrap_or(1).max(1);
        let size = q.page_size.unwrap_or(20).clamp(1, 100);
        let offset = ((page - 1) * size) as i64;

        let total: (i64,) = order_count_query(q)
            .build_query_as()
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;

        let rows = order_page_query(q, size as i64, offset)
            .build_query_as::<HubOrderSummary>()
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok((rows, total.0.max(0)))
    }

    /// 中枢侧订单汇总合计（看板口径：单量 + 订单额 + 实付额）。
    pub async fn sum_order_summaries(
        &self,
        link_id: Option<i32>,
    ) -> Result<(i64, rust_decimal::Decimal, rust_decimal::Decimal)> {
        let row: (i64, Option<rust_decimal::Decimal>, Option<rust_decimal::Decimal>) =
            sqlx::query_as(
                "SELECT COUNT(*), SUM(order_amount), SUM(paid_amount) FROM hub_order_summary WHERE ($1::int IS NULL OR link_id = $1)",
            )
            .bind(link_id)
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;
        Ok((
            row.0.max(0),
            row.1.unwrap_or_default(),
            row.2.unwrap_or_default(),
        ))
    }

    /// 成员侧上报端：按 order_id 游标增量扫本地有效订单头（件数 = 数量合计）。
    pub async fn list_local_orders_after(
        &self,
        cursor: i32,
        limit: i64,
    ) -> Result<Vec<LocalOrderRow>> {
        sqlx::query_as::<_, LocalOrderRow>(
            "SELECT o.order_id, o.order_sn, o.user_id, o.shop_id, COALESCE(s.shop_name, '') AS shop_name, o.order_status, o.pay_status, o.total_amount, o.paid_amount, COALESCE((SELECT SUM(oi.quantity) FROM order_item oi WHERE oi.order_id = o.order_id), 0)::int AS item_count, o.add_time, o.pay_time, GREATEST(o.add_time, o.pay_time, o.shipping_time, o.received_time) AS version_time FROM \"order\" o LEFT JOIN shop s ON s.shop_id = o.shop_id WHERE o.order_id > $1 AND o.is_valid = 1 AND o.is_del = 0 ORDER BY o.order_id ASC LIMIT $2",
        )
        .bind(cursor)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 成员侧上报端：重扫窗口内有状态推进的有效订单（游标之后的变更靠版本戳发现）。
    pub async fn list_local_orders_changed_since(
        &self,
        since_version_time: i32,
        max_order_id: i32,
        limit: i64,
    ) -> Result<Vec<LocalOrderRow>> {
        sqlx::query_as::<_, LocalOrderRow>(
            "SELECT o.order_id, o.order_sn, o.user_id, o.shop_id, COALESCE(s.shop_name, '') AS shop_name, o.order_status, o.pay_status, o.total_amount, o.paid_amount, COALESCE((SELECT SUM(oi.quantity) FROM order_item oi WHERE oi.order_id = o.order_id), 0)::int AS item_count, o.add_time, o.pay_time, GREATEST(o.add_time, o.pay_time, o.shipping_time, o.received_time) AS version_time FROM \"order\" o LEFT JOIN shop s ON s.shop_id = o.shop_id WHERE o.order_id <= $2 AND o.is_valid = 1 AND o.is_del = 0 AND GREATEST(o.add_time, o.pay_time, o.shipping_time, o.received_time) > $1 ORDER BY o.order_id ASC LIMIT $3",
        )
        .bind(since_version_time)
        .bind(max_order_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }
}

impl HubFederationRepository {
    // ———————————————————— 三期①：登录互认（迁移 039） ————————————————————

    /// 中枢侧落一条授权请求（待同意）。
    pub async fn insert_sso_request(
        &self,
        link_id: i32,
        state: &str,
        return_url: &str,
        now: i32,
    ) -> Result<i32> {
        let row: (i32,) = sqlx::query_as(
            "INSERT INTO hub_sso_request (link_id, state, return_url, status, add_time, update_time)
             VALUES ($1, $2, $3, 0, $4, $4) RETURNING id",
        )
        .bind(link_id)
        .bind(state)
        .bind(return_url)
        .bind(now)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.0)
    }

    pub async fn find_sso_request(&self, id: i32) -> Result<Option<HubSsoRequest>> {
        sqlx::query_as::<_, HubSsoRequest>("SELECT * FROM hub_sso_request WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    pub async fn find_sso_request_by_code_hash(
        &self,
        code_hash: &str,
    ) -> Result<Option<HubSsoRequest>> {
        sqlx::query_as::<_, HubSsoRequest>(
            "SELECT * FROM hub_sso_request WHERE code_hash = $1 AND code_hash <> ''",
        )
        .bind(code_hash)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 同意落码：仅待同意（status=0）可流转，条件更新防并发重复同意。
    #[allow(clippy::too_many_arguments)]
    pub async fn approve_sso_request(
        &self,
        id: i32,
        hub_user_id: i32,
        global_member_id: &str,
        code_hash: &str,
        code_expire_time: i32,
        now: i32,
    ) -> Result<bool> {
        let n = sqlx::query(
            "UPDATE hub_sso_request SET status = 1, hub_user_id = $2, global_member_id = $3, code_hash = $4, code_expire_time = $5, code_used = 0, update_time = $6 WHERE id = $1 AND status = 0",
        )
        .bind(id)
        .bind(hub_user_id)
        .bind(global_member_id)
        .bind(code_hash)
        .bind(code_expire_time)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(n.rows_affected() > 0)
    }

    /// 拒绝：仅待同意可流转。
    pub async fn deny_sso_request(&self, id: i32, now: i32) -> Result<bool> {
        let n = sqlx::query(
            "UPDATE hub_sso_request SET status = 2, update_time = $2 WHERE id = $1 AND status = 0",
        )
        .bind(id)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(n.rows_affected() > 0)
    }

    /// 置过期（读时惰性流转）。
    pub async fn expire_sso_request(&self, id: i32, now: i32) -> Result<()> {
        sqlx::query(
            "UPDATE hub_sso_request SET status = 3, update_time = $2 WHERE id = $1 AND status = 0",
        )
        .bind(id)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    /// 兑换烧码：未用 → 已用，条件更新保证一次性（并发兑换只有一个成功）。
    pub async fn burn_sso_code(&self, id: i32, now: i32) -> Result<bool> {
        let n = sqlx::query(
            "UPDATE hub_sso_request SET code_used = 1, update_time = $2 WHERE id = $1 AND status = 1 AND code_used = 0",
        )
        .bind(id)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(n.rows_affected() > 0)
    }

    /// 管理端：SSO 请求分页（授权审计）。
    pub async fn list_sso_requests(
        &self,
        q: &HubSsoListQuery,
    ) -> Result<(Vec<HubSsoRequest>, i64)> {
        let page = q.page.unwrap_or(1).max(1);
        let size = q.page_size.unwrap_or(20).clamp(1, 100);
        let offset = ((page - 1) * size) as i64;

        let total: (i64,) = sso_count_query(q)
            .build_query_as()
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;
        let rows = sso_page_query(q, size as i64, offset)
            .build_query_as::<HubSsoRequest>()
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok((rows, total.0.max(0)))
    }

    /// 成员侧落发起记录（state 只存哈希）。
    pub async fn insert_sso_state(
        &self,
        link_id: i32,
        state_hash: &str,
        now: i32,
    ) -> Result<i32> {
        let row: (i32,) = sqlx::query_as(
            "INSERT INTO hub_sso_state (link_id, state_hash, status, add_time, update_time)
             VALUES ($1, $2, 0, $3, $3) RETURNING id",
        )
        .bind(link_id)
        .bind(state_hash)
        .bind(now)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.0)
    }

    pub async fn find_sso_state_by_hash(
        &self,
        state_hash: &str,
    ) -> Result<Option<HubSsoState>> {
        sqlx::query_as::<_, HubSsoState>("SELECT * FROM hub_sso_state WHERE state_hash = $1")
            .bind(state_hash)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 回调落号完成：置已完成并记本地会员。
    pub async fn complete_sso_state(
        &self,
        state_hash: &str,
        local_user_id: i32,
        now: i32,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE hub_sso_state SET status = 1, local_user_id = $2, update_time = $3 WHERE state_hash = $1",
        )
        .bind(state_hash)
        .bind(local_user_id)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }
}

impl HubFederationRepository {
    // ———————————————————— 三期②：配置分发（迁移 043） ————————————————————

    /// 当前最大版本号（发布取号与拉取水位）。
    pub async fn max_config_version(&self) -> Result<i32> {
        let row: (i32,) =
            sqlx::query_as("SELECT COALESCE(MAX(version), 0) FROM hub_config_package")
                .fetch_one(&self.pool)
                .await
                .map_err(map_err)?;
        Ok(row.0)
    }

    /// 发布落包（version 由服务层取 max+1 传入，UNIQUE(version) 兜底防撞）。
    #[allow(clippy::too_many_arguments)]
    pub async fn insert_config_package(
        &self,
        package_type: &str,
        title: &str,
        version: i32,
        payload: &str,
        content_hash: &str,
        apply_mode: i32,
        effective_from: i32,
        effective_to: i32,
        now: i32,
    ) -> Result<i32> {
        let row: (i32,) = sqlx::query_as(
            "INSERT INTO hub_config_package (package_type, title, version, payload, content_hash, apply_mode, effective_from, effective_to, status, add_time, update_time)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 1, $9, $9) RETURNING id",
        )
        .bind(package_type)
        .bind(title)
        .bind(version)
        .bind(payload)
        .bind(content_hash)
        .bind(apply_mode as i16)
        .bind(effective_from)
        .bind(effective_to)
        .bind(now)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.0)
    }

    pub async fn find_config_package(&self, id: i32) -> Result<Option<HubConfigPackage>> {
        sqlx::query_as::<_, HubConfigPackage>("SELECT * FROM hub_config_package WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 撤销包（仅生效中可撤）。
    pub async fn revoke_config_package(&self, id: i32, now: i32) -> Result<bool> {
        let n = sqlx::query(
            "UPDATE hub_config_package SET status = 0, update_time = $2 WHERE id = $1 AND status = 1",
        )
        .bind(id)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(n.rows_affected() > 0)
    }

    /// 成员拉取：version 大于水位、生效中、窗口内的包（按 version 升序）。
    pub async fn list_effective_config_packages(
        &self,
        since_version: i32,
        now: i32,
        limit: i64,
    ) -> Result<Vec<HubConfigPackage>> {
        sqlx::query_as::<_, HubConfigPackage>(
            "SELECT * FROM hub_config_package WHERE status = 1 AND version > $1 AND (effective_from = 0 OR effective_from <= $2) AND (effective_to = 0 OR effective_to >= $2) ORDER BY version ASC LIMIT $3",
        )
        .bind(since_version)
        .bind(now)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 心跳点亮判定：该连接是否存在「版本高于其最大已回执版本」的生效包。
    pub async fn has_unacked_config_package(&self, link_id: i32, now: i32) -> Result<bool> {
        let row: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM hub_config_package p WHERE p.status = 1 AND (p.effective_from = 0 OR p.effective_from <= $2) AND (p.effective_to = 0 OR p.effective_to >= $2) AND p.version > COALESCE((SELECT MAX(a.version) FROM hub_config_ack a WHERE a.link_id = $1), 0)",
        )
        .bind(link_id)
        .bind(now)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.0 > 0)
    }

    /// 回执 upsert（一连接一包一行，后报覆盖先报）。
    pub async fn upsert_config_ack(
        &self,
        link_id: i32,
        package_id: i32,
        version: i32,
        ack_status: i32,
        message: &str,
        now: i32,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO hub_config_ack (link_id, package_id, version, ack_status, message, add_time, update_time)
             VALUES ($1, $2, $3, $4, $5, $6, $6)
             ON CONFLICT (link_id, package_id) DO UPDATE SET version = EXCLUDED.version, ack_status = EXCLUDED.ack_status, message = EXCLUDED.message, update_time = EXCLUDED.update_time",
        )
        .bind(link_id)
        .bind(package_id)
        .bind(version)
        .bind(ack_status as i16)
        .bind(message)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    /// 包的回执覆盖统计：(回执连接数, 已应用数)。
    pub async fn config_ack_counts(&self, package_id: i32) -> Result<(i64, i64)> {
        let row: (i64, i64) = sqlx::query_as(
            "SELECT COUNT(*), COALESCE(SUM(CASE WHEN ack_status = 1 THEN 1 ELSE 0 END), 0) FROM hub_config_ack WHERE package_id = $1",
        )
        .bind(package_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok((row.0.max(0), row.1.max(0)))
    }

    pub async fn list_config_packages(
        &self,
        q: &HubConfigListQuery,
    ) -> Result<(Vec<HubConfigPackage>, i64)> {
        let page = q.page.unwrap_or(1).max(1);
        let size = q.page_size.unwrap_or(20).clamp(1, 100);
        let offset = ((page - 1) * size) as i64;
        let total: (i64,) = config_count_query(q)
            .build_query_as()
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;
        let rows = config_page_query(q, size as i64, offset)
            .build_query_as::<HubConfigPackage>()
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok((rows, total.0.max(0)))
    }

    pub async fn list_config_acks(
        &self,
        q: &HubConfigAckListQuery,
    ) -> Result<(Vec<HubConfigAck>, i64)> {
        let page = q.page.unwrap_or(1).max(1);
        let size = q.page_size.unwrap_or(20).clamp(1, 100);
        let offset = ((page - 1) * size) as i64;
        let total: (i64,) = config_ack_count_query(q)
            .build_query_as()
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;
        let rows = config_ack_page_query(q, size as i64, offset)
            .build_query_as::<HubConfigAck>()
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok((rows, total.0.max(0)))
    }

    /// 成员侧落地 upsert：新包落待处理；已存在的包只刷新快照字段，
    /// 应用状态不回退（已应用/已拒绝的包重拉不复活）。
    #[allow(clippy::too_many_arguments)]
    pub async fn upsert_config_apply(
        &self,
        link_id: i32,
        package_id: i32,
        version: i32,
        package_type: &str,
        title: &str,
        payload: &str,
        content_hash: &str,
        apply_mode: i32,
        now: i32,
    ) -> Result<HubConfigApply> {
        sqlx::query_as::<_, HubConfigApply>(
            "INSERT INTO hub_config_apply (link_id, package_id, version, package_type, title, payload, content_hash, apply_mode, apply_status, apply_message, add_time, update_time)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 0, '', $9, $9)
             ON CONFLICT (link_id, package_id) DO UPDATE SET version = EXCLUDED.version, package_type = EXCLUDED.package_type, title = EXCLUDED.title, payload = EXCLUDED.payload, content_hash = EXCLUDED.content_hash, apply_mode = EXCLUDED.apply_mode, update_time = EXCLUDED.update_time
             RETURNING *",
        )
        .bind(link_id)
        .bind(package_id)
        .bind(version)
        .bind(package_type)
        .bind(title)
        .bind(payload)
        .bind(content_hash)
        .bind(apply_mode as i16)
        .bind(now)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)
    }

    pub async fn find_config_apply(&self, id: i32) -> Result<Option<HubConfigApply>> {
        sqlx::query_as::<_, HubConfigApply>("SELECT * FROM hub_config_apply WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 落地状态流转（仅待处理可流转，防重复确认）。
    pub async fn set_config_apply_status(
        &self,
        id: i32,
        status: i32,
        message: &str,
        now: i32,
    ) -> Result<bool> {
        let n = sqlx::query(
            "UPDATE hub_config_apply SET apply_status = $2, apply_message = $3, update_time = $4 WHERE id = $1 AND apply_status = 0",
        )
        .bind(id)
        .bind(status as i16)
        .bind(message)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(n.rows_affected() > 0)
    }

    /// 本机对某连接的落地最大版本（拉取水位）。
    pub async fn max_apply_version(&self, link_id: i32) -> Result<i32> {
        let row: (i32,) = sqlx::query_as(
            "SELECT COALESCE(MAX(version), 0) FROM hub_config_apply WHERE link_id = $1",
        )
        .bind(link_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.0)
    }

    pub async fn list_config_applies(
        &self,
        q: &HubConfigApplyListQuery,
    ) -> Result<(Vec<HubConfigApply>, i64)> {
        let page = q.page.unwrap_or(1).max(1);
        let size = q.page_size.unwrap_or(20).clamp(1, 100);
        let offset = ((page - 1) * size) as i64;
        let total: (i64,) = config_apply_count_query(q)
            .build_query_as()
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;
        let rows = config_apply_page_query(q, size as i64, offset)
            .build_query_as::<HubConfigApply>()
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok((rows, total.0.max(0)))
    }
}

fn config_count_query(q: &HubConfigListQuery) -> QueryBuilder<'static, Postgres> {
    let mut qb: QueryBuilder<'static, Postgres> =
        QueryBuilder::new("SELECT COUNT(*) FROM hub_config_package WHERE 1=1");
    push_config_filters(&mut qb, q);
    qb
}

fn config_page_query(
    q: &HubConfigListQuery,
    size: i64,
    offset: i64,
) -> QueryBuilder<'static, Postgres> {
    let mut qb: QueryBuilder<'static, Postgres> =
        QueryBuilder::new("SELECT * FROM hub_config_package WHERE 1=1");
    push_config_filters(&mut qb, q);
    qb.push(" ORDER BY version DESC LIMIT ");
    qb.push_bind(size);
    qb.push(" OFFSET ");
    qb.push_bind(offset);
    qb
}

fn push_config_filters(qb: &mut QueryBuilder<'static, Postgres>, q: &HubConfigListQuery) {
    if let Some(t) = q.package_type.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        qb.push(" AND package_type = ");
        qb.push_bind(t.to_string());
    }
    if let Some(status) = q.status {
        qb.push(" AND status = ");
        qb.push_bind(status as i16);
    }
}

fn config_ack_count_query(q: &HubConfigAckListQuery) -> QueryBuilder<'static, Postgres> {
    let mut qb: QueryBuilder<'static, Postgres> =
        QueryBuilder::new("SELECT COUNT(*) FROM hub_config_ack WHERE 1=1");
    push_config_ack_filters(&mut qb, q);
    qb
}

fn config_ack_page_query(
    q: &HubConfigAckListQuery,
    size: i64,
    offset: i64,
) -> QueryBuilder<'static, Postgres> {
    let mut qb: QueryBuilder<'static, Postgres> =
        QueryBuilder::new("SELECT * FROM hub_config_ack WHERE 1=1");
    push_config_ack_filters(&mut qb, q);
    qb.push(" ORDER BY id DESC LIMIT ");
    qb.push_bind(size);
    qb.push(" OFFSET ");
    qb.push_bind(offset);
    qb
}

fn push_config_ack_filters(qb: &mut QueryBuilder<'static, Postgres>, q: &HubConfigAckListQuery) {
    if let Some(package_id) = q.package_id.filter(|v| *v > 0) {
        qb.push(" AND package_id = ");
        qb.push_bind(package_id);
    }
    if let Some(link_id) = q.link_id.filter(|v| *v > 0) {
        qb.push(" AND link_id = ");
        qb.push_bind(link_id);
    }
}

fn config_apply_count_query(q: &HubConfigApplyListQuery) -> QueryBuilder<'static, Postgres> {
    let mut qb: QueryBuilder<'static, Postgres> =
        QueryBuilder::new("SELECT COUNT(*) FROM hub_config_apply WHERE 1=1");
    push_config_apply_filters(&mut qb, q);
    qb
}

fn config_apply_page_query(
    q: &HubConfigApplyListQuery,
    size: i64,
    offset: i64,
) -> QueryBuilder<'static, Postgres> {
    let mut qb: QueryBuilder<'static, Postgres> =
        QueryBuilder::new("SELECT * FROM hub_config_apply WHERE 1=1");
    push_config_apply_filters(&mut qb, q);
    qb.push(" ORDER BY version DESC LIMIT ");
    qb.push_bind(size);
    qb.push(" OFFSET ");
    qb.push_bind(offset);
    qb
}

fn push_config_apply_filters(
    qb: &mut QueryBuilder<'static, Postgres>,
    q: &HubConfigApplyListQuery,
) {
    if let Some(link_id) = q.link_id.filter(|v| *v > 0) {
        qb.push(" AND link_id = ");
        qb.push_bind(link_id);
    }
    if let Some(status) = q.apply_status {
        qb.push(" AND apply_status = ");
        qb.push_bind(status as i16);
    }
}

fn sso_count_query(q: &HubSsoListQuery) -> QueryBuilder<'static, Postgres> {
    let mut qb: QueryBuilder<'static, Postgres> =
        QueryBuilder::new("SELECT COUNT(*) FROM hub_sso_request WHERE 1=1");
    push_sso_filters(&mut qb, q);
    qb
}

fn sso_page_query(
    q: &HubSsoListQuery,
    size: i64,
    offset: i64,
) -> QueryBuilder<'static, Postgres> {
    let mut qb: QueryBuilder<'static, Postgres> =
        QueryBuilder::new("SELECT * FROM hub_sso_request WHERE 1=1");
    push_sso_filters(&mut qb, q);
    qb.push(" ORDER BY id DESC LIMIT ");
    qb.push_bind(size);
    qb.push(" OFFSET ");
    qb.push_bind(offset);
    qb
}

fn push_sso_filters(qb: &mut QueryBuilder<'static, Postgres>, q: &HubSsoListQuery) {
    if let Some(link_id) = q.link_id.filter(|v| *v > 0) {
        qb.push(" AND link_id = ");
        qb.push_bind(link_id);
    }
    if let Some(status) = q.status {
        qb.push(" AND status = ");
        qb.push_bind(status as i16);
    }
}

// ———————————————————— 列表查询构造（纯函数，供单测断言占位符序列） ————————————————————
//
// 占位符只由 QueryBuilder 的 push_bind 生成：条件片段与绑定必须交错 push，
// 种子文本只含无占位符的 `WHERE 1=1`。严禁把 $n 预先烧进种子再 push_bind——
// push_bind 按自身参数计数把占位符追加到串尾，与种子内预置的 $n 冲突错位，
// 曾致四个列表接口带任一过滤即 500（`syntax error at or near "$1"`）。
// 同口径先例：catalog/product_feed.rs 的钥列表构造。

fn cred_count_query(q: &HubCredentialListQuery) -> QueryBuilder<'static, Postgres> {
    let mut qb: QueryBuilder<'static, Postgres> =
        QueryBuilder::new("SELECT COUNT(*) FROM hub_credential WHERE 1=1");
    push_cred_filters(&mut qb, q);
    qb
}

fn cred_page_query(
    q: &HubCredentialListQuery,
    size: i64,
    offset: i64,
) -> QueryBuilder<'static, Postgres> {
    let mut qb: QueryBuilder<'static, Postgres> =
        QueryBuilder::new("SELECT * FROM hub_credential WHERE 1=1");
    push_cred_filters(&mut qb, q);
    qb.push(" ORDER BY credential_id DESC LIMIT ");
    qb.push_bind(size);
    qb.push(" OFFSET ");
    qb.push_bind(offset);
    qb
}

fn link_count_query(q: &HubLinkListQuery) -> QueryBuilder<'static, Postgres> {
    let mut qb: QueryBuilder<'static, Postgres> =
        QueryBuilder::new("SELECT COUNT(*) FROM hub_link WHERE 1=1");
    push_link_filters(&mut qb, q);
    qb
}

fn link_page_query(
    q: &HubLinkListQuery,
    size: i64,
    offset: i64,
) -> QueryBuilder<'static, Postgres> {
    let mut qb: QueryBuilder<'static, Postgres> =
        QueryBuilder::new("SELECT * FROM hub_link WHERE 1=1");
    push_link_filters(&mut qb, q);
    qb.push(" ORDER BY link_id DESC LIMIT ");
    qb.push_bind(size);
    qb.push(" OFFSET ");
    qb.push_bind(offset);
    qb
}

fn log_count_query(q: &HubCallLogListQuery) -> QueryBuilder<'static, Postgres> {
    let mut qb: QueryBuilder<'static, Postgres> =
        QueryBuilder::new("SELECT COUNT(*) FROM hub_call_log WHERE 1=1");
    push_log_filters(&mut qb, q);
    qb
}

fn log_page_query(
    q: &HubCallLogListQuery,
    size: i64,
    offset: i64,
) -> QueryBuilder<'static, Postgres> {
    let mut qb: QueryBuilder<'static, Postgres> =
        QueryBuilder::new("SELECT * FROM hub_call_log WHERE 1=1");
    push_log_filters(&mut qb, q);
    qb.push(" ORDER BY log_id DESC LIMIT ");
    qb.push_bind(size);
    qb.push(" OFFSET ");
    qb.push_bind(offset);
    qb
}

fn map_count_query(q: &HubAggregationListQuery) -> QueryBuilder<'static, Postgres> {
    let mut qb: QueryBuilder<'static, Postgres> =
        QueryBuilder::new("SELECT COUNT(*) FROM hub_product_map WHERE 1=1");
    push_map_filters(&mut qb, q);
    qb
}

fn map_page_query(
    q: &HubAggregationListQuery,
    size: i64,
    offset: i64,
) -> QueryBuilder<'static, Postgres> {
    let mut qb: QueryBuilder<'static, Postgres> =
        QueryBuilder::new("SELECT * FROM hub_product_map WHERE 1=1");
    push_map_filters(&mut qb, q);
    qb.push(" ORDER BY map_id DESC LIMIT ");
    qb.push_bind(size);
    qb.push(" OFFSET ");
    qb.push_bind(offset);
    qb
}

fn push_cred_filters(qb: &mut QueryBuilder<'_, Postgres>, q: &HubCredentialListQuery) {
    if let Some(status) = q.status {
        qb.push(" AND status = ");
        qb.push_bind(status);
    }
    if let Some(conn_type) = q.conn_type {
        qb.push(" AND conn_type = ");
        qb.push_bind(conn_type);
    }
    if let Some(kw) = q.keyword.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        let like = format!("%{kw}%");
        qb.push(" AND (cred_name ILIKE ");
        qb.push_bind(like.clone());
        qb.push(" OR access_key ILIKE ");
        qb.push_bind(like);
        qb.push(")");
    }
}

fn push_link_filters(qb: &mut QueryBuilder<'_, Postgres>, q: &HubLinkListQuery) {
    if let Some(status) = q.status {
        qb.push(" AND status = ");
        qb.push_bind(status);
    }
    if let Some(role) = q.role {
        qb.push(" AND role = ");
        qb.push_bind(role);
    }
    if let Some(kw) = q.keyword.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        let like = format!("%{kw}%");
        qb.push(" AND (peer_name ILIKE ");
        qb.push_bind(like.clone());
        qb.push(" OR peer_base_url ILIKE ");
        qb.push_bind(like.clone());
        qb.push(" OR peer_instance_uuid ILIKE ");
        qb.push_bind(like);
        qb.push(")");
    }
}

fn push_log_filters(qb: &mut QueryBuilder<'_, Postgres>, q: &HubCallLogListQuery) {
    if let Some(id) = q.credential_id.filter(|id| *id > 0) {
        qb.push(" AND credential_id = ");
        qb.push_bind(id);
    }
    if let Some(id) = q.link_id.filter(|id| *id > 0) {
        qb.push(" AND link_id = ");
        qb.push_bind(id);
    }
}

fn push_map_filters(qb: &mut QueryBuilder<'_, Postgres>, q: &HubAggregationListQuery) {
    if let Some(id) = q.link_id.filter(|id| *id > 0) {
        qb.push(" AND link_id = ");
        qb.push_bind(id);
    }
    if let Some(state) = q.sync_state {
        qb.push(" AND sync_state = ");
        qb.push_bind(state);
    }
    if let Some(kw) = q.keyword.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        let like = format!("%{kw}%");
        qb.push(" AND (product_name ILIKE ");
        qb.push_bind(like.clone());
        qb.push(" OR remote_shop_name ILIKE ");
        qb.push_bind(like);
        qb.push(")");
    }
}

fn member_count_query(q: &HubMemberListQuery) -> QueryBuilder<'static, Postgres> {
    let mut qb: QueryBuilder<'static, Postgres> =
        QueryBuilder::new("SELECT COUNT(*) FROM hub_member_link WHERE 1=1");
    push_member_filters(&mut qb, q);
    qb
}

fn member_page_query(
    q: &HubMemberListQuery,
    size: i64,
    offset: i64,
) -> QueryBuilder<'static, Postgres> {
    let mut qb: QueryBuilder<'static, Postgres> =
        QueryBuilder::new("SELECT * FROM hub_member_link WHERE 1=1");
    push_member_filters(&mut qb, q);
    qb.push(" ORDER BY id DESC LIMIT ");
    qb.push_bind(size);
    qb.push(" OFFSET ");
    qb.push_bind(offset);
    qb
}

fn push_member_filters(qb: &mut QueryBuilder<'_, Postgres>, q: &HubMemberListQuery) {
    if let Some(id) = q.link_id.filter(|id| *id > 0) {
        qb.push(" AND link_id = ");
        qb.push_bind(id);
    }
    if let Some(status) = q.status {
        qb.push(" AND status = ");
        qb.push_bind(status);
    }
    if let Some(kw) = q.keyword.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        let like = format!("%{kw}%");
        qb.push(" AND (global_member_id ILIKE ");
        qb.push_bind(like.clone());
        qb.push(" OR nickname ILIKE ");
        qb.push_bind(like);
        qb.push(")");
    }
}

fn conflict_count_query(q: &HubMemberConflictListQuery) -> QueryBuilder<'static, Postgres> {
    let mut qb: QueryBuilder<'static, Postgres> =
        QueryBuilder::new("SELECT COUNT(*) FROM hub_member_conflict WHERE 1=1");
    push_conflict_filters(&mut qb, q);
    qb
}

fn conflict_page_query(
    q: &HubMemberConflictListQuery,
    size: i64,
    offset: i64,
) -> QueryBuilder<'static, Postgres> {
    let mut qb: QueryBuilder<'static, Postgres> =
        QueryBuilder::new("SELECT * FROM hub_member_conflict WHERE 1=1");
    push_conflict_filters(&mut qb, q);
    qb.push(" ORDER BY id DESC LIMIT ");
    qb.push_bind(size);
    qb.push(" OFFSET ");
    qb.push_bind(offset);
    qb
}

fn push_conflict_filters(qb: &mut QueryBuilder<'_, Postgres>, q: &HubMemberConflictListQuery) {
    if let Some(id) = q.link_id.filter(|id| *id > 0) {
        qb.push(" AND link_id = ");
        qb.push_bind(id);
    }
    if let Some(status) = q.status {
        qb.push(" AND status = ");
        qb.push_bind(status);
    }
}

fn order_count_query(q: &HubOrderSummaryListQuery) -> QueryBuilder<'static, Postgres> {
    let mut qb: QueryBuilder<'static, Postgres> =
        QueryBuilder::new("SELECT COUNT(*) FROM hub_order_summary WHERE 1=1");
    push_order_filters(&mut qb, q);
    qb
}

fn order_page_query(
    q: &HubOrderSummaryListQuery,
    size: i64,
    offset: i64,
) -> QueryBuilder<'static, Postgres> {
    let mut qb: QueryBuilder<'static, Postgres> =
        QueryBuilder::new("SELECT * FROM hub_order_summary WHERE 1=1");
    push_order_filters(&mut qb, q);
    qb.push(" ORDER BY id DESC LIMIT ");
    qb.push_bind(size);
    qb.push(" OFFSET ");
    qb.push_bind(offset);
    qb
}

fn push_order_filters(qb: &mut QueryBuilder<'_, Postgres>, q: &HubOrderSummaryListQuery) {
    if let Some(id) = q.link_id.filter(|id| *id > 0) {
        qb.push(" AND link_id = ");
        qb.push_bind(id);
    }
    if let Some(kw) = q.keyword.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        qb.push(" AND remote_order_sn ILIKE ");
        qb.push_bind(format!("%{kw}%"));
    }
}

fn map_err(e: sqlx::Error) -> Error {
    match e {
        sqlx::Error::RowNotFound => Error::NotFound("记录不存在".into()),
        other => Error::Database(other.to_string()),
    }
}

#[cfg(test)]
mod list_sql_tests {
    use super::*;

    /// 按出现顺序抽出 SQL 文本中的 `$n` 占位符编号序列。
    fn placeholder_seq(sql: &str) -> Vec<u32> {
        let mut seq = Vec::new();
        let mut chars = sql.chars().peekable();
        while let Some(ch) = chars.next() {
            if ch == '$' {
                let mut num = String::new();
                while let Some(&c) = chars.peek() {
                    if c.is_ascii_digit() {
                        num.push(c);
                        chars.next();
                    } else {
                        break;
                    }
                }
                if !num.is_empty() {
                    seq.push(num.parse().expect("占位符编号必为数字"));
                }
            }
        }
        seq
    }

    /// 占位符序列必须恰为 `$1..=$binds` 连续递增：占位符个数与绑定数一致、
    /// 无重复无跳号。旧构造（$n 烧种子 + push_bind 追加串尾）生成的序列形如
    /// `$1,$1,$2…`，在此断言下必红——本组测试即为该缺陷的回归钉。
    fn assert_placeholders(sql: &str, binds: u32) {
        let expected: Vec<u32> = (1..=binds).collect();
        assert_eq!(placeholder_seq(sql), expected, "sql: {sql}");
    }

    // ———— 凭证列表 ————

    #[test]
    fn cred_list_no_filter() {
        let q = HubCredentialListQuery::default();
        assert_eq!(
            cred_count_query(&q).sql(),
            "SELECT COUNT(*) FROM hub_credential WHERE 1=1"
        );
        assert_placeholders(cred_count_query(&q).sql(), 0);
        assert_placeholders(cred_page_query(&q, 20, 0).sql(), 2);
    }

    #[test]
    fn cred_list_all_filters() {
        let q = HubCredentialListQuery {
            status: Some(1),
            conn_type: Some(3),
            keyword: Some("hub".into()),
            ..Default::default()
        };
        // status + conn_type + keyword×2 = 4 个过滤绑定；分页再 +2。
        assert_placeholders(cred_count_query(&q).sql(), 4);
        assert_placeholders(cred_page_query(&q, 20, 0).sql(), 6);
    }

    #[test]
    fn cred_list_blank_keyword_is_no_filter() {
        let q = HubCredentialListQuery {
            keyword: Some("   ".into()),
            ..Default::default()
        };
        assert_placeholders(cred_count_query(&q).sql(), 0);
        assert_placeholders(cred_page_query(&q, 20, 0).sql(), 2);
    }

    // ———— 连接列表（验收 §3 撞 500 的原点） ————

    #[test]
    fn link_list_no_filter() {
        let q = HubLinkListQuery::default();
        assert_placeholders(link_count_query(&q).sql(), 0);
        assert_placeholders(link_page_query(&q, 20, 0).sql(), 2);
    }

    #[test]
    fn link_list_all_filters() {
        let q = HubLinkListQuery {
            status: Some(1),
            role: Some(2),
            keyword: Some("peer".into()),
            ..Default::default()
        };
        // status + role + keyword×3 = 5 个过滤绑定；分页再 +2。
        assert_placeholders(link_count_query(&q).sql(), 5);
        assert_eq!(
            link_page_query(&q, 20, 40).sql(),
            "SELECT * FROM hub_link WHERE 1=1 AND status = $1 AND role = $2 \
             AND (peer_name ILIKE $3 OR peer_base_url ILIKE $4 OR peer_instance_uuid ILIKE $5) \
             ORDER BY link_id DESC LIMIT $6 OFFSET $7"
        );
    }

    #[test]
    fn link_list_keyword_only() {
        let q = HubLinkListQuery {
            keyword: Some("peer".into()),
            ..Default::default()
        };
        assert_placeholders(link_count_query(&q).sql(), 3);
        assert_placeholders(link_page_query(&q, 20, 0).sql(), 5);
    }

    // ———— 调用日志列表 ————

    #[test]
    fn log_list_no_filter() {
        let q = HubCallLogListQuery::default();
        assert_placeholders(log_count_query(&q).sql(), 0);
        assert_placeholders(log_page_query(&q, 20, 0).sql(), 2);
    }

    #[test]
    fn log_list_all_filters() {
        let q = HubCallLogListQuery {
            credential_id: Some(9),
            link_id: Some(3),
            ..Default::default()
        };
        assert_placeholders(log_count_query(&q).sql(), 2);
        assert_placeholders(log_page_query(&q, 20, 0).sql(), 4);
    }

    #[test]
    fn log_list_nonpositive_ids_are_no_filter() {
        let q = HubCallLogListQuery {
            credential_id: Some(0),
            link_id: Some(-1),
            ..Default::default()
        };
        assert_placeholders(log_count_query(&q).sql(), 0);
        assert_placeholders(log_page_query(&q, 20, 0).sql(), 2);
    }

    // ———— 聚合映射列表（含删除路径） ————

    #[test]
    fn map_list_no_filter() {
        let q = HubAggregationListQuery::default();
        assert_placeholders(map_count_query(&q).sql(), 0);
        assert_placeholders(map_page_query(&q, 20, 0).sql(), 2);
    }

    #[test]
    fn map_list_all_filters() {
        let q = HubAggregationListQuery {
            link_id: Some(7),
            sync_state: Some(1),
            keyword: Some("shop".into()),
            ..Default::default()
        };
        // link_id + sync_state + keyword×2 = 4 个过滤绑定；分页再 +2。
        assert_placeholders(map_count_query(&q).sql(), 4);
        assert_placeholders(map_page_query(&q, 20, 0).sql(), 6);
    }

    #[test]
    fn map_list_link_id_only() {
        // link_id 单过滤形态（聚合列表按连接查看映射的常用形态）；
        // 该过滤形态单独钉死：1 个过滤绑定。
        let q = HubAggregationListQuery {
            link_id: Some(7),
            ..Default::default()
        };
        assert_placeholders(map_count_query(&q).sql(), 1);
        assert_eq!(
            map_page_query(&q, 100, 0).sql(),
            "SELECT * FROM hub_product_map WHERE 1=1 AND link_id = $1 \
             ORDER BY map_id DESC LIMIT $2 OFFSET $3"
        );
    }

    // ———— 二期：会员映射 / 冲突 / 订单汇总列表 ————

    #[test]
    fn member_list_no_filter() {
        let q = HubMemberListQuery::default();
        assert_placeholders(member_count_query(&q).sql(), 0);
        assert_placeholders(member_page_query(&q, 20, 0).sql(), 2);
    }

    #[test]
    fn member_list_all_filters() {
        let q = HubMemberListQuery {
            link_id: Some(3),
            status: Some(1),
            keyword: Some("gm_".into()),
            ..Default::default()
        };
        // link_id + status + keyword×2 = 4 个过滤绑定；分页再 +2。
        assert_placeholders(member_count_query(&q).sql(), 4);
        assert_placeholders(member_page_query(&q, 20, 0).sql(), 6);
    }

    #[test]
    fn conflict_list_filters() {
        let q = HubMemberConflictListQuery {
            link_id: Some(3),
            status: Some(0),
            ..Default::default()
        };
        assert_placeholders(conflict_count_query(&q).sql(), 2);
        assert_placeholders(conflict_page_query(&q, 20, 0).sql(), 4);
        let q = HubMemberConflictListQuery::default();
        assert_placeholders(conflict_count_query(&q).sql(), 0);
        assert_placeholders(conflict_page_query(&q, 20, 0).sql(), 2);
    }

    #[test]
    fn order_list_filters() {
        let q = HubOrderSummaryListQuery {
            link_id: Some(3),
            keyword: Some("SN2026".into()),
            ..Default::default()
        };
        assert_placeholders(order_count_query(&q).sql(), 2);
        assert_placeholders(order_page_query(&q, 20, 0).sql(), 4);
        let q = HubOrderSummaryListQuery::default();
        assert_placeholders(order_count_query(&q).sql(), 0);
        assert_placeholders(order_page_query(&q, 20, 0).sql(), 2);
    }

    #[test]
    fn config_list_filters() {
        let q = HubConfigListQuery {
            package_type: Some("announcement".into()),
            status: Some(1),
            ..Default::default()
        };
        assert_placeholders(config_count_query(&q).sql(), 2);
        assert_placeholders(config_page_query(&q, 20, 0).sql(), 4);
        let q = HubConfigAckListQuery {
            package_id: Some(3),
            link_id: Some(2),
            ..Default::default()
        };
        assert_placeholders(config_ack_count_query(&q).sql(), 2);
        assert_placeholders(config_ack_page_query(&q, 20, 0).sql(), 4);
        let q = HubConfigApplyListQuery {
            link_id: Some(2),
            apply_status: Some(0),
            ..Default::default()
        };
        assert_placeholders(config_apply_count_query(&q).sql(), 2);
        assert_placeholders(config_apply_page_query(&q, 20, 0).sql(), 4);
    }

    #[test]
    fn sso_list_filters() {
        let q = HubSsoListQuery {
            link_id: Some(3),
            status: Some(1),
            ..Default::default()
        };
        assert_placeholders(sso_count_query(&q).sql(), 2);
        assert_placeholders(sso_page_query(&q, 20, 0).sql(), 4);
        let q = HubSsoListQuery::default();
        assert_placeholders(sso_count_query(&q).sql(), 0);
        assert_placeholders(sso_page_query(&q, 20, 0).sql(), 2);
    }
}
