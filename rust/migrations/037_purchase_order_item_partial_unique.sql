-- ============================================================
-- 037_purchase_order_item_partial_unique.sql
-- 采购单明细唯一索引改 partial（仅未删行唯一）：purchase_order_item
-- 的 (order_id, vendor_product_sku_id) 唯一索引由部署脚本
-- 2026-11-10_unique_keys.sql 建为全量唯一、不带 is_del 条件。
-- 草稿改明细走「软删旧明细 + 重插」：同一 (order_id, sku) 的第二行
-- （新行 is_del=0、旧行 is_del=1）被全量唯一索引拦截，事务回滚、
-- 草稿保存必失败。本迁移把唯一性收窄到未删行：软删行不再占用
-- 唯一键，活动行唯一性不变（存量活动行在全量唯一约束下本就无
-- 重复，partial 索引可直接建成）。
-- 部署顺序：须先于引用草稿改明细的新二进制执行（旧二进制不依赖
-- 索引形态，反序亦安全）。
-- 注意：迁移执行器按分号逐条执行且会剥离 -- 注释行，语句内勿写分号。
-- ============================================================

DROP INDEX IF EXISTS idx_purchase_order_item_uk_purchase_order_item_sk;

CREATE UNIQUE INDEX IF NOT EXISTS idx_purchase_order_item_uk_active ON purchase_order_item (order_id, vendor_product_sku_id) WHERE is_del = 0;
