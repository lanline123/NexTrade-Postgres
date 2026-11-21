-- ============================================================
-- 025_delivery_time_slot.sql
-- 同城配送送达时间选择（论坛帖 467744 + 预约配送子项 297743，
-- 我方新增，用户 2026-11-21 拍板 B 类缓做全部开工）：同城配送
-- 结算时可选送达日期 + 时间段（今日时段 / 未来预约），商家按
-- 时段备货。时段定义挂门店同城配送配置（增量列），订单侧不
-- 侵入订单状态/金额结构，独立落 order_delivery_slot 一行（挂
-- 买家下单的根订单，拆单子单经 parent_order_id 解析，与
-- order_gift 同模式）。
-- 口径：开关默认关（存量店铺行为不变）；开启后同城下单必选
-- 时段；今日时段须满足截单提前量（时段开始前 cutoff 分钟停
-- 止接单）；时段容量按 (门店, 时段开始时间戳) 统计未取消的
-- 有效订单，capacity = 0 表示不限量。
-- 部署顺序：本迁移必须先于新二进制换版执行（新版会读写新列
-- 与新表，旧版对新列/新表不敏感，反序安全、正序必需）。
-- 注意：迁移执行器按分号逐条执行且会剥离 -- 注释行，语句内勿写分号。
-- ============================================================

ALTER TABLE shop_local_delivery_config ADD COLUMN IF NOT EXISTS time_slot_enabled INT NOT NULL DEFAULT 0;

ALTER TABLE shop_local_delivery_config ADD COLUMN IF NOT EXISTS time_slots TEXT NOT NULL DEFAULT '';

ALTER TABLE shop_local_delivery_config ADD COLUMN IF NOT EXISTS booking_days INT NOT NULL DEFAULT 2;

ALTER TABLE shop_local_delivery_config ADD COLUMN IF NOT EXISTS cutoff_minutes INT NOT NULL DEFAULT 30;

CREATE TABLE IF NOT EXISTS order_delivery_slot (
    order_id INT PRIMARY KEY,
    order_sn VARCHAR(64) NOT NULL DEFAULT '',
    shop_id INT NOT NULL DEFAULT 0,
    slot_start INT NOT NULL DEFAULT 0,
    slot_end INT NOT NULL DEFAULT 0,
    slot_text VARCHAR(64) NOT NULL DEFAULT '',
    add_time INT NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_order_delivery_slot_shop_start ON order_delivery_slot (shop_id, slot_start);
