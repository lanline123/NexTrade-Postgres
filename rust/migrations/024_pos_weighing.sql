-- ============================================================
-- 024_pos_weighing.sql
-- POS 称重商品与蓝牙秤（论坛 B 类缓做项第 21 条，帖 127845 + 727834，
-- 用户 2026-11-21 21:29 拍板缓做项全部开工，我方新增，原版无此形态）。
-- 口径：product.is_weighing = 1 的商品在 POS 按重量计价，product_price
-- 为每个计价单位的价格，weigh_unit_grams 为计价单位克数（默认 500，
-- 即元/500g），行金额 = 单价 × 重量克数 ÷ 单位克数，四舍五入到分。
-- pos_pending_order_item 三列为称重行快照（重量/单价/单位克数），
-- 非称重行恒为 NULL，存量与普通行行为零变化。
-- 编号说明：仓库现行最大 015，同批并行线已占用 016，本线取 017，
-- 代码零引用迁移号，合并改序只需改文件名。
-- 部署顺序：本迁移必须先于新二进制执行（新版实体含新列且保存链会
-- 写新列，旧版对多出的列不敏感，反序安全、正序必需）。
-- 注意：迁移执行器按分号逐条执行且会剥离注释行，语句内勿写分号。
-- ============================================================

ALTER TABLE product ADD COLUMN IF NOT EXISTS is_weighing INT NOT NULL DEFAULT 0;
ALTER TABLE product ADD COLUMN IF NOT EXISTS weigh_unit_grams INT NOT NULL DEFAULT 500;
ALTER TABLE pos_pending_order_item ADD COLUMN IF NOT EXISTS weight_grams INT;
ALTER TABLE pos_pending_order_item ADD COLUMN IF NOT EXISTS weigh_unit_price NUMERIC(12, 2);
ALTER TABLE pos_pending_order_item ADD COLUMN IF NOT EXISTS weigh_unit_grams INT;
