-- ============================================================
-- 013_points_exchange_coupon.sql（原暂存编号 012 与礼品卡适用范围迁移撞号，主聊发卡时改序为 013）
-- 积分兑换优惠券（论坛 B 类第 1 项，帖 517907；用户 2026-11-21
-- 20:43 拍板「做」）：points_exchange 增加兑换类型与券模板关联。
-- 原版积分兑换仅商品型，券型为我方新增。
-- exchange_type：1 = 商品（存量行为零变化），2 = 优惠券。
-- coupon_id：券型行的券模板 ID（coupon.coupon_id），商品型为 NULL。
-- 券型行 product_id 约定存 0（列保持 NOT NULL，与 sku_id=0 /
-- shop_id=0 的既有哨兵口径一致，实体与既有查询零改动面）。
-- 部署顺序：本迁移必须先于新二进制换版执行（新版实体含两列，
-- 旧版对多出的列不敏感，反序安全、正序必需）。
-- 注意：迁移执行器按分号逐条执行且会剥离 -- 注释行，语句内勿写分号。
-- ============================================================

ALTER TABLE points_exchange ADD COLUMN IF NOT EXISTS exchange_type INT NOT NULL DEFAULT 1;

ALTER TABLE points_exchange ADD COLUMN IF NOT EXISTS coupon_id INT NULL;
