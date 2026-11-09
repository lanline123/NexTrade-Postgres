//! Schema 一致性测试。
//!
//! **目的**：防止实体定义与真实数据库 schema 发生漂移。
//!
//! 背景：本项目的实体由 `tools/schema_parser.py` 从
//! `java/database/pure.sql` + 增量补丁自动生成，字段名与可空性必须
//! 严格对齐真实库表。早期曾出现实体字段名与真实列名不符
//! （如 `ShippingTpl.tpl_id` 实际应为 `shipping_tpl_id`）却通过了
//! `cargo check` 与全部单元测试的情况 —— 因为仓储层用运行时
//! `query_as::<_, T>()`，SQL 列名不参与编译校验。
//!
//! 本测试通过读取 `tests/schema_snapshot.txt`（由
//! `tools/gen_schema_snapshot.py` 从权威 schema 导出），
//! 断言关键实体**确实声明了**这些字段，且序列化名与列名一致。
//!
//! 快照过期时重新生成：
//! ```powershell
//! python tools/gen_schema_snapshot.py
//! ```

use std::collections::HashMap;

/// 解析快照文件为 `表名 -> Vec<(列名, Rust 类型声明)>`。
fn load_snapshot() -> HashMap<String, Vec<(String, String)>> {
    let raw = include_str!("schema_snapshot.txt");
    let mut out: HashMap<String, Vec<(String, String)>> = HashMap::new();
    let mut current: Option<String> = None;

    for line in raw.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            let name = line[1..line.len() - 1].to_string();
            current = Some(name.clone());
            out.entry(name).or_default();
            continue;
        }
        if let Some(table) = &current {
            if let Some((col, ty)) = line.split_once('|') {
                out.get_mut(table)
                    .expect("表条目应已初始化")
                    .push((col.to_string(), ty.to_string()));
            }
        }
    }
    out
}

/// 用 serde 序列化一个实例，得到实际字段名集合。
///
/// 通过 `#[sqlx(rename_all = "snake_case")]` + serde 默认命名，
/// 序列化键即列名（两者在本项目中保持一致）。
fn field_names<T: serde::Serialize + Default>() -> Vec<String> {
    let value = serde_json::to_value(T::default()).expect("实体应可序列化");
    match value {
        serde_json::Value::Object(map) => map.keys().cloned().collect(),
        other => panic!("实体应序列化为 JSON 对象，实际为 {other:?}"),
    }
}

/// 断言实体的字段集合与快照列集合一致。
fn assert_matches<T: serde::Serialize + Default>(table: &str, snapshot: &HashMap<String, Vec<(String, String)>>) {
    let expected: Vec<String> = snapshot
        .get(table)
        .unwrap_or_else(|| panic!("快照缺少表 {table}"))
        .iter()
        .map(|(c, _)| c.clone())
        .collect();

    let mut actual = field_names::<T>();
    let mut want = expected.clone();
    actual.sort();
    want.sort();

    // 实体可以比快照多出业务派生字段（如 exist=false 的扩展字段），
    // 但**不允许缺少**快照中的列 —— 缺少即意味着读不到该列数据。
    let missing: Vec<&String> = want.iter().filter(|c| !actual.contains(c)).collect();
    assert!(
        missing.is_empty(),
        "实体 {table} 缺少 schema 中的列: {missing:?}\n\
         这会导致该列数据无法读取，或 SQL 引用报 Unknown column。\n\
         请运行 `python tools/schema_parser.py --gen-entities` 重新生成实体。"
    );
}

#[test]
fn admin_user_matches_schema() {
    let snap = load_snapshot();
    assert_matches::<nextrade_core::entities::AdminUser>("admin_user", &snap);
}

#[test]
fn user_matches_schema() {
    let snap = load_snapshot();
    assert_matches::<nextrade_core::entities::User>("user", &snap);
}

#[test]
fn product_matches_schema() {
    let snap = load_snapshot();
    assert_matches::<nextrade_core::entities::Product>("product", &snap);
}

#[test]
fn product_sku_matches_schema() {
    let snap = load_snapshot();
    assert_matches::<nextrade_core::entities::ProductSku>("product_sku", &snap);
}

#[test]
fn order_matches_schema() {
    let snap = load_snapshot();
    assert_matches::<nextrade_core::entities::Order>("order", &snap);
}

#[test]
fn order_item_matches_schema() {
    let snap = load_snapshot();
    assert_matches::<nextrade_core::entities::OrderItem>("order_item", &snap);
}

#[test]
fn cart_matches_schema() {
    let snap = load_snapshot();
    assert_matches::<nextrade_core::entities::Cart>("cart", &snap);
}

#[test]
fn category_matches_schema() {
    let snap = load_snapshot();
    assert_matches::<nextrade_core::entities::Category>("category", &snap);
}

#[test]
fn brand_matches_schema() {
    let snap = load_snapshot();
    assert_matches::<nextrade_core::entities::Brand>("brand", &snap);
}

#[test]
fn coupon_matches_schema() {
    let snap = load_snapshot();
    assert_matches::<nextrade_core::entities::Coupon>("coupon", &snap);
}

#[test]
fn user_coupon_matches_schema() {
    let snap = load_snapshot();
    assert_matches::<nextrade_core::entities::UserCoupon>("user_coupon", &snap);
}

#[test]
fn article_matches_schema() {
    let snap = load_snapshot();
    assert_matches::<nextrade_core::entities::Article>("article", &snap);
}

#[test]
fn article_category_matches_schema() {
    let snap = load_snapshot();
    assert_matches::<nextrade_core::entities::ArticleCategory>("article_category", &snap);
}

#[test]
fn logistics_company_matches_schema() {
    let snap = load_snapshot();
    assert_matches::<nextrade_core::entities::LogisticsCompany>("logistics_company", &snap);
}

#[test]
fn shipping_tpl_matches_schema() {
    let snap = load_snapshot();
    assert_matches::<nextrade_core::entities::ShippingTpl>("shipping_tpl", &snap);
}

#[test]
fn shipping_tpl_info_matches_schema() {
    let snap = load_snapshot();
    assert_matches::<nextrade_core::entities::ShippingTplInfo>("shipping_tpl_info", &snap);
}

#[test]
fn shipping_type_matches_schema() {
    let snap = load_snapshot();
    assert_matches::<nextrade_core::entities::ShippingType>("shipping_type", &snap);
}

#[test]
fn member_tag_matches_schema() {
    let snap = load_snapshot();
    assert_matches::<nextrade_core::entities::MemberTag>("member_tag", &snap);
}

#[test]
fn member_tag_group_matches_schema() {
    let snap = load_snapshot();
    assert_matches::<nextrade_core::entities::MemberTagGroup>("member_tag_group", &snap);
}

#[test]
fn user_rank_matches_schema() {
    let snap = load_snapshot();
    assert_matches::<nextrade_core::entities::UserRank>("user_rank", &snap);
}

#[test]
fn user_points_log_matches_schema() {
    let snap = load_snapshot();
    assert_matches::<nextrade_core::entities::UserPointsLog>("user_points_log", &snap);
}

#[test]
fn seckill_matches_schema() {
    let snap = load_snapshot();
    assert_matches::<nextrade_core::entities::Seckill>("seckill", &snap);
}

#[test]
fn seckill_item_matches_schema() {
    let snap = load_snapshot();
    assert_matches::<nextrade_core::entities::SeckillItem>("seckill_item", &snap);
}

#[test]
fn time_discount_matches_schema() {
    let snap = load_snapshot();
    assert_matches::<nextrade_core::entities::TimeDiscount>("time_discount", &snap);
}

#[test]
fn time_discount_item_matches_schema() {
    let snap = load_snapshot();
    assert_matches::<nextrade_core::entities::TimeDiscountItem>("time_discount_item", &snap);
}

#[test]
fn paylog_matches_schema() {
    let snap = load_snapshot();
    assert_matches::<nextrade_core::entities::Paylog>("paylog", &snap);
}

#[test]
fn paylog_refund_matches_schema() {
    let snap = load_snapshot();
    assert_matches::<nextrade_core::entities::PaylogRefund>("paylog_refund", &snap);
}

#[test]
fn refund_apply_matches_schema() {
    let snap = load_snapshot();
    assert_matches::<nextrade_core::entities::RefundApply>("refund_apply", &snap);
}

#[test]
fn refund_log_matches_schema() {
    let snap = load_snapshot();
    assert_matches::<nextrade_core::entities::RefundLog>("refund_log", &snap);
}

/// 快照本身应非空且格式正确（防止快照文件被清空后测试形同虚设）。
#[test]
fn snapshot_is_wellformed() {
    let snap = load_snapshot();
    assert!(
        snap.len() >= 20,
        "快照应至少包含 20 张表，实际 {}",
        snap.len()
    );
    for (table, cols) in &snap {
        assert!(!cols.is_empty(), "表 {table} 不应为空");
        for (col, ty) in cols {
            assert!(!col.is_empty(), "表 {table} 存在空列名");
            assert!(
                ty.starts_with("Option<") || matches!(
                    ty.as_str(),
                    "i32" | "i64" | "String" | "Decimal"
                ),
                "表 {table}.{col} 类型声明异常: {ty}"
            );
        }
    }
}