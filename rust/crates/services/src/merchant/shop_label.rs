//! 店铺标签服务（merchant::shop_label）—— **本平台新增**（论坛帖
//! 207878955346177229「建议能给店铺增加标签，类似淘宝那种」）。
//!
//! 原版 tigshop 无此功能（官方回复称尚在设计中）。模型：
//! - `shop_label`：平台标签库（名称 / 颜色 / 排序 / 启停），平台管理员维护；
//! - `shop_label_rel`：店铺 ↔ 标签分配（平台打标，整店替换式保存）；
//! - 买家侧在店铺名旁展示（店铺街 / 店铺首页 / 商品详情店铺卡，PC + H5）。
//!
//! 与既有 `store_tip`（门店服务提示，如「支持自提」，前台字段 `tipNames`）
//! 是两套概念，互不影响。自动榜单类标签（如按销量自动授标）不在本期范围，
//! 见实施报告的二期说明。
//!
//! 迁移：`rust/deploy/pg/2026-11-21_shop_label.sql`（部署前必须先执行）。

use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

use nextrade_core::models::merchant::shop_client::ShopLabelVO;
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::Repositories;

/// 全局业务错误码（与 shop_client 的 SERVICE_DATA_ERROR 口径一致）。
const SERVICE_DATA_ERROR: i32 = 1001;

/// 标签库一行（管理端全字段）。
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ShopLabelItem {
    pub label_id: i32,
    pub label_name: String,
    pub color: String,
    pub sort_order: i32,
    /// 1 启用 0 停用（停用后买家侧不展示，分配关系保留）
    pub status: i32,
    pub add_time: i32,
}

/// 标签库写参数（create / update 共用）。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ShopLabelWrite {
    pub label_id: Option<i32>,
    pub label_name: Option<String>,
    pub color: Option<String>,
    pub sort_order: Option<i32>,
    pub status: Option<i32>,
}

/// 一条分配关系（管理端全量分配视图）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ShopLabelAssignment {
    pub shop_id: i32,
    pub label_id: i32,
    pub label_name: String,
    pub color: String,
}

#[derive(Debug, FromRow)]
struct LabelJoinRow {
    shop_id: i32,
    label_id: i32,
    label_name: String,
    color: String,
}

fn now_i32() -> i32 {
    i32::try_from(nextrade_common::utils::now_seconds()).unwrap_or(0)
}

/// 批量取多家店铺的**启用中**标签（按标签排序、id 稳定排序）。
/// 前台各列表/详情共用；表未迁移时 SQL 直接报错——部署顺序由迁移约束保证。
pub async fn fetch_labels_for_shops(
    pool: &sqlx::PgPool,
    shop_ids: &[i32],
) -> Result<HashMap<i32, Vec<ShopLabelVO>>> {
    let mut map: HashMap<i32, Vec<ShopLabelVO>> = HashMap::new();
    if shop_ids.is_empty() {
        return Ok(map);
    }
    let rows: Vec<LabelJoinRow> = sqlx::query_as(
        "SELECT r.shop_id AS shop_id, l.label_id AS label_id, \
                l.label_name AS label_name, l.color AS color \
         FROM shop_label_rel r \
         JOIN shop_label l ON l.label_id = r.label_id \
         WHERE l.status = 1 AND r.shop_id = ANY($1) \
         ORDER BY l.sort_order ASC, l.label_id ASC",
    )
    .bind(shop_ids)
    .fetch_all(pool)
    .await.map_err(|e| Error::Database(e.to_string()))?;
    for row in rows {
        map.entry(row.shop_id).or_default().push(ShopLabelVO {
            label_id: Some(row.label_id),
            label_name: Some(row.label_name),
            color: if row.color.is_empty() {
                None
            } else {
                Some(row.color)
            },
        });
    }
    Ok(map)
}

/// 单店启用中标签。
pub async fn fetch_labels_for_shop(
    pool: &sqlx::PgPool,
    shop_id: i32,
) -> Result<Vec<ShopLabelVO>> {
    let mut map = fetch_labels_for_shops(pool, &[shop_id]).await?;
    Ok(map.remove(&shop_id).unwrap_or_default())
}

pub struct ShopLabelService {
    repos: Arc<Repositories>,
}

impl ShopLabelService {
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    fn pool(&self) -> &sqlx::PgPool {
        &self.repos.pool
    }

    /// 标签库全量（含停用），按排序、id 升序。
    pub async fn list_labels(&self) -> Result<Vec<ShopLabelItem>> {
        let rows: Vec<ShopLabelItem> = sqlx::query_as(
            "SELECT label_id, label_name, color, sort_order, status, add_time \
             FROM shop_label ORDER BY sort_order ASC, label_id ASC",
        )
        .fetch_all(self.pool())
        .await.map_err(|e| Error::Database(e.to_string()))?;
        Ok(rows)
    }

    pub async fn create_label(&self, p: &ShopLabelWrite) -> Result<i32> {
        let name = validate_name(p.label_name.as_deref())?;
        let color = validate_color(p.color.as_deref())?;
        let row: (i32,) = sqlx::query_as(
            "INSERT INTO shop_label (label_name, color, sort_order, status, add_time) \
             VALUES ($1, $2, $3, $4, $5) RETURNING label_id",
        )
        .bind(name)
        .bind(color)
        .bind(p.sort_order.unwrap_or(0))
        .bind(if p.status == Some(0) { 0 } else { 1 })
        .bind(now_i32())
        .fetch_one(self.pool())
        .await.map_err(|e| Error::Database(e.to_string()))?;
        Ok(row.0)
    }

    pub async fn update_label(&self, p: &ShopLabelWrite) -> Result<()> {
        let label_id = p
            .label_id
            .filter(|id| *id > 0)
            .ok_or_else(|| Error::biz(SERVICE_DATA_ERROR, "标签ID不能为空"))?;
        let name = validate_name(p.label_name.as_deref())?;
        let color = validate_color(p.color.as_deref())?;
        let affected = sqlx::query(
            "UPDATE shop_label SET label_name = $2, color = $3, sort_order = $4, status = $5 \
             WHERE label_id = $1",
        )
        .bind(label_id)
        .bind(name)
        .bind(color)
        .bind(p.sort_order.unwrap_or(0))
        .bind(if p.status == Some(0) { 0 } else { 1 })
        .execute(self.pool())
        .await.map_err(|e| Error::Database(e.to_string()))?
        .rows_affected();
        if affected == 0 {
            return Err(Error::biz(SERVICE_DATA_ERROR, "标签不存在"));
        }
        Ok(())
    }

    /// 删除标签（连同全部分配关系，事务内完成）。
    pub async fn delete_label(&self, label_id: i32) -> Result<()> {
        if label_id <= 0 {
            return Err(Error::biz(SERVICE_DATA_ERROR, "标签ID不能为空"));
        }
        let mut tx = self.pool().begin().await.map_err(|e| Error::Database(e.to_string()))?;
        sqlx::query("DELETE FROM shop_label_rel WHERE label_id = $1")
            .bind(label_id)
            .execute(&mut *tx)
            .await.map_err(|e| Error::Database(e.to_string()))?;
        let affected = sqlx::query("DELETE FROM shop_label WHERE label_id = $1")
            .bind(label_id)
            .execute(&mut *tx)
            .await.map_err(|e| Error::Database(e.to_string()))?
            .rows_affected();
        if affected == 0 {
            return Err(Error::biz(SERVICE_DATA_ERROR, "标签不存在"));
        }
        tx.commit().await.map_err(|e| Error::Database(e.to_string()))?;
        Ok(())
    }

    /// 全量分配关系（管理端店铺列表一次性拉取渲染徽章，避免逐店请求）。
    pub async fn all_assignments(&self) -> Result<Vec<ShopLabelAssignment>> {
        let rows: Vec<LabelJoinRow> = sqlx::query_as(
            "SELECT r.shop_id AS shop_id, l.label_id AS label_id, \
                    l.label_name AS label_name, l.color AS color \
             FROM shop_label_rel r \
             JOIN shop_label l ON l.label_id = r.label_id \
             ORDER BY l.sort_order ASC, l.label_id ASC",
        )
        .fetch_all(self.pool())
        .await.map_err(|e| Error::Database(e.to_string()))?;
        Ok(rows
            .into_iter()
            .map(|r| ShopLabelAssignment {
                shop_id: r.shop_id,
                label_id: r.label_id,
                label_name: r.label_name,
                color: r.color,
            })
            .collect())
    }

    /// 单店已分配的标签 id（管理端回显，含已停用标签）。
    pub async fn shop_label_ids(&self, shop_id: i32) -> Result<Vec<i32>> {
        let rows: Vec<(i32,)> =
            sqlx::query_as("SELECT label_id FROM shop_label_rel WHERE shop_id = $1 ORDER BY label_id ASC")
                .bind(shop_id)
                .fetch_all(self.pool())
                .await.map_err(|e| Error::Database(e.to_string()))?;
        Ok(rows.into_iter().map(|(id,)| id).collect())
    }

    /// 整店替换式保存分配：校验店铺与标签存在后，事务内先删后插。
    pub async fn set_shop_labels(&self, shop_id: i32, label_ids: &[i32]) -> Result<()> {
        if shop_id <= 0 {
            return Err(Error::biz(SERVICE_DATA_ERROR, "店铺ID不能为空"));
        }
        let shop_exists: Option<(i32,)> = sqlx::query_as(
            "SELECT shop_id FROM shop WHERE shop_id = $1 \
             AND (is_delete IS NULL OR is_delete = 0)",
        )
        .bind(shop_id)
        .fetch_optional(self.pool())
        .await.map_err(|e| Error::Database(e.to_string()))?;
        if shop_exists.is_none() {
            return Err(Error::biz(SERVICE_DATA_ERROR, "店铺不存在"));
        }
        let mut unique: Vec<i32> = Vec::new();
        for id in label_ids {
            if *id > 0 && !unique.contains(id) {
                unique.push(*id);
            }
        }
        if unique.len() > 10 {
            return Err(Error::biz(SERVICE_DATA_ERROR, "每家店铺最多分配 10 个标签"));
        }
        if !unique.is_empty() {
            let found: Vec<(i32,)> =
                sqlx::query_as("SELECT label_id FROM shop_label WHERE label_id = ANY($1)")
                    .bind(&unique)
                    .fetch_all(self.pool())
                    .await.map_err(|e| Error::Database(e.to_string()))?;
            if found.len() != unique.len() {
                return Err(Error::biz(SERVICE_DATA_ERROR, "所选标签不存在"));
            }
        }
        let mut tx = self.pool().begin().await.map_err(|e| Error::Database(e.to_string()))?;
        sqlx::query("DELETE FROM shop_label_rel WHERE shop_id = $1")
            .bind(shop_id)
            .execute(&mut *tx)
            .await.map_err(|e| Error::Database(e.to_string()))?;
        for id in &unique {
            sqlx::query(
                "INSERT INTO shop_label_rel (shop_id, label_id) VALUES ($1, $2) \
                 ON CONFLICT (shop_id, label_id) DO NOTHING",
            )
            .bind(shop_id)
            .bind(id)
            .execute(&mut *tx)
            .await.map_err(|e| Error::Database(e.to_string()))?;
        }
        tx.commit().await.map_err(|e| Error::Database(e.to_string()))?;
        Ok(())
    }
}

fn validate_name(raw: Option<&str>) -> Result<String> {
    let name = raw.map(str::trim).unwrap_or("");
    if name.is_empty() {
        return Err(Error::biz(SERVICE_DATA_ERROR, "标签名称不能为空"));
    }
    if name.chars().count() > 10 {
        return Err(Error::biz(SERVICE_DATA_ERROR, "标签名称最多 10 个字"));
    }
    Ok(name.to_string())
}

fn validate_color(raw: Option<&str>) -> Result<String> {
    let color = raw.map(str::trim).unwrap_or("");
    if color.is_empty() {
        return Ok(String::new());
    }
    let valid = color.len() == 7
        && color.starts_with('#')
        && color[1..].chars().all(|c| c.is_ascii_hexdigit());
    if !valid {
        return Err(Error::biz(SERVICE_DATA_ERROR, "颜色须为 #RRGGBB 格式"));
    }
    Ok(color.to_string())
}

#[cfg(test)]
mod tests {
    use super::{validate_color, validate_name};

    #[test]
    fn name_rules() {
        assert!(validate_name(Some(" 金牌商家 ")).is_ok());
        assert!(validate_name(Some("")).is_err());
        assert!(validate_name(None).is_err());
        assert!(validate_name(Some("一二三四五六七八九十甲")).is_err());
    }

    #[test]
    fn color_rules() {
        assert_eq!(validate_color(None).unwrap(), "");
        assert_eq!(validate_color(Some("#E93B3D")).unwrap(), "#E93B3D");
        assert!(validate_color(Some("red")).is_err());
        assert!(validate_color(Some("#12345")).is_err());
    }
}
