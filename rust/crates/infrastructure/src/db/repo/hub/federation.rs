//! 平台联邦仓储（hub::federation）。
//!
//! 表（`rust/deploy/pg/2026-11-20_hub_federation.sql`）：
//! - `hub_credential` + 店铺/商品/分类/覆盖 4 子表
//! - `hub_link`（类型③连接，兼实例注册表）
//! - `hub_pairing_code`、`hub_call_log`、`hub_product_map`
//!
//! SQL 一律运行时 `sqlx::query` / `query_as`，`$n` 占位。

use sqlx::{PgPool, Postgres, QueryBuilder};

use nextrade_core::models::hub::federation::{
    HubCallLogListQuery, HubCredential, HubCredentialListQuery, HubLink, HubLinkListQuery,
    HubPairingCode, HubPriceOverride, HubProductMap, HubAggregationListQuery,
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
        let mut cond = String::from(" WHERE 1=1");
        if q.status.is_some() {
            cond.push_str(" AND status = ?");
        }
        if q.conn_type.is_some() {
            cond.push_str(" AND conn_type = ?");
        }
        if q.keyword.as_deref().is_some_and(|k| !k.trim().is_empty()) {
            cond.push_str(" AND (cred_name ILIKE ? OR access_key ILIKE ?)");
        }
        let sql_cond = pg_placeholders(&cond);

        let mut qb: QueryBuilder<'_, Postgres> =
            QueryBuilder::new(format!("SELECT COUNT(*) FROM hub_credential{sql_cond}"));
        push_cred_filters(&mut qb, q);
        let total: (i64,) = qb
            .build_query_as()
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;

        let mut qb: QueryBuilder<'_, Postgres> = QueryBuilder::new(format!(
            "SELECT * FROM hub_credential{sql_cond} ORDER BY credential_id DESC LIMIT "
        ));
        push_cred_filters(&mut qb, q);
        qb.push_bind(size as i64);
        qb.push(" OFFSET ");
        qb.push_bind(offset);
        let rows = qb
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
        for table in [
            "hub_credential_shop",
            "hub_credential_product",
            "hub_credential_category",
            "hub_credential_price_override",
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
        let mut cond = String::from(" WHERE 1=1");
        if q.status.is_some() {
            cond.push_str(" AND status = ?");
        }
        if q.role.is_some() {
            cond.push_str(" AND role = ?");
        }
        if q.keyword.as_deref().is_some_and(|k| !k.trim().is_empty()) {
            cond.push_str(" AND (peer_name ILIKE ? OR peer_base_url ILIKE ? OR peer_instance_uuid ILIKE ?)");
        }
        let sql_cond = pg_placeholders(&cond);

        let mut qb: QueryBuilder<'_, Postgres> =
            QueryBuilder::new(format!("SELECT COUNT(*) FROM hub_link{sql_cond}"));
        push_link_filters(&mut qb, q);
        let total: (i64,) = qb
            .build_query_as()
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;

        let mut qb: QueryBuilder<'_, Postgres> = QueryBuilder::new(format!(
            "SELECT * FROM hub_link{sql_cond} ORDER BY link_id DESC LIMIT "
        ));
        push_link_filters(&mut qb, q);
        qb.push_bind(size as i64);
        qb.push(" OFFSET ");
        qb.push_bind(offset);
        let rows = qb
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

    pub async fn delete_link(&self, link_id: i32) -> Result<()> {
        sqlx::query("DELETE FROM hub_link WHERE link_id = $1")
            .bind(link_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
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
        let mut cond = String::from(" WHERE 1=1");
        if q.credential_id.is_some_and(|id| id > 0) {
            cond.push_str(" AND credential_id = ?");
        }
        if q.link_id.is_some_and(|id| id > 0) {
            cond.push_str(" AND link_id = ?");
        }
        let sql_cond = pg_placeholders(&cond);
        let mut qb: QueryBuilder<'_, Postgres> =
            QueryBuilder::new(format!("SELECT COUNT(*) FROM hub_call_log{sql_cond}"));
        push_log_filters(&mut qb, q);
        let total: (i64,) = qb
            .build_query_as()
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;
        let mut qb: QueryBuilder<'_, Postgres> = QueryBuilder::new(format!(
            "SELECT * FROM hub_call_log{sql_cond} ORDER BY log_id DESC LIMIT "
        ));
        push_log_filters(&mut qb, q);
        qb.push_bind(size as i64);
        qb.push(" OFFSET ");
        qb.push_bind(offset);
        let rows = qb
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
        let mut cond = String::from(" WHERE 1=1");
        if q.link_id.is_some_and(|id| id > 0) {
            cond.push_str(" AND link_id = ?");
        }
        if q.sync_state.is_some() {
            cond.push_str(" AND sync_state = ?");
        }
        if q.keyword.as_deref().is_some_and(|k| !k.trim().is_empty()) {
            cond.push_str(" AND (product_name ILIKE ? OR remote_shop_name ILIKE ?)");
        }
        let sql_cond = pg_placeholders(&cond);
        let mut qb: QueryBuilder<'_, Postgres> =
            QueryBuilder::new(format!("SELECT COUNT(*) FROM hub_product_map{sql_cond}"));
        push_map_filters(&mut qb, q);
        let total: (i64,) = qb
            .build_query_as()
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;
        let mut qb: QueryBuilder<'_, Postgres> = QueryBuilder::new(format!(
            "SELECT * FROM hub_product_map{sql_cond} ORDER BY map_id DESC LIMIT "
        ));
        push_map_filters(&mut qb, q);
        qb.push_bind(size as i64);
        qb.push(" OFFSET ");
        qb.push_bind(offset);
        let rows = qb
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
}

fn push_cred_filters(qb: &mut QueryBuilder<'_, Postgres>, q: &HubCredentialListQuery) {
    if let Some(status) = q.status {
        qb.push_bind(status);
    }
    if let Some(conn_type) = q.conn_type {
        qb.push_bind(conn_type);
    }
    if let Some(kw) = q.keyword.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        let like = format!("%{kw}%");
        qb.push_bind(like.clone());
        qb.push_bind(like);
    }
}

fn push_link_filters(qb: &mut QueryBuilder<'_, Postgres>, q: &HubLinkListQuery) {
    if let Some(status) = q.status {
        qb.push_bind(status);
    }
    if let Some(role) = q.role {
        qb.push_bind(role);
    }
    if let Some(kw) = q.keyword.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        let like = format!("%{kw}%");
        qb.push_bind(like.clone());
        qb.push_bind(like.clone());
        qb.push_bind(like);
    }
}

fn push_log_filters(qb: &mut QueryBuilder<'_, Postgres>, q: &HubCallLogListQuery) {
    if let Some(id) = q.credential_id.filter(|id| *id > 0) {
        qb.push_bind(id);
    }
    if let Some(id) = q.link_id.filter(|id| *id > 0) {
        qb.push_bind(id);
    }
}

fn push_map_filters(qb: &mut QueryBuilder<'_, Postgres>, q: &HubAggregationListQuery) {
    if let Some(id) = q.link_id.filter(|id| *id > 0) {
        qb.push_bind(id);
    }
    if let Some(state) = q.sync_state {
        qb.push_bind(state);
    }
    if let Some(kw) = q.keyword.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        let like = format!("%{kw}%");
        qb.push_bind(like.clone());
        qb.push_bind(like);
    }
}

/// 把模板里的 `?` 顺序替换为 PG 的 `$1..$n`（模板内不得出现字面 `?`）。
fn pg_placeholders(template: &str) -> String {
    let mut out = String::with_capacity(template.len() + 8);
    let mut n = 0;
    for ch in template.chars() {
        if ch == '?' {
            n += 1;
            out.push_str(&format!("${n}"));
        } else {
            out.push(ch);
        }
    }
    out
}

fn map_err(e: sqlx::Error) -> Error {
    match e {
        sqlx::Error::RowNotFound => Error::NotFound("记录不存在".into()),
        other => Error::Database(other.to_string()),
    }
}
