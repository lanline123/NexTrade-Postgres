-- 2026-11-18: 默认规格（Tigshop v5.8.63 商品规格展示优化）。
-- product_sku 加 is_default 标志：同一商品至多一个启用 SKU 为默认，
-- 详情页打开时自动预选该规格（价格 / 库存 / 规格图随所选 SKU 联动）。
-- 互斥由服务层保存时清零兜底（sync_skus 收尾），不加唯一索引
-- （禁用行 / 历史数据可能残留多行 = 1，清零逻辑按启用行优先收敛）。
-- 幂等：ADD COLUMN IF NOT EXISTS，可重复执行。
BEGIN;

ALTER TABLE product_sku ADD COLUMN IF NOT EXISTS is_default integer NOT NULL DEFAULT 0;

COMMIT;
