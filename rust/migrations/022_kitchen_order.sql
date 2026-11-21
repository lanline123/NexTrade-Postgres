-- ============================================================
-- 022_kitchen_order.sql
-- 后厨出单与催菜（论坛帖 817927，我方新增，用户 2026-11-21
-- 拍板 B 类缓做 28 项全部开工）：餐饮/到店履约场景下，订单
-- 付款后进入后厨队列，后厨在后台「后厨出单」看板上做状态
-- 流转（待制作 → 制作中 → 已出单）；买家可在订单详情催菜，
-- 催菜落商家消息中心（新消息类型 58）并在后厨看板高亮。
-- 后厨子状态不侵入订单状态枚举，独立落 order_kitchen 一行
-- （挂履约订单：未拆单为根订单、拆单后为各子单），无行时
-- 语义即「待制作」；催菜流水落 order_kitchen_urge 供频控
-- （同单同买家 10 分钟冷却 + 每日上限 5 次）与审计。
-- 部署顺序：本迁移必须先于新二进制换版执行（新版会读写
-- 两张新表，旧版对新表不敏感，反序安全、正序必需）。
-- 注意：迁移执行器按分号逐条执行且会剥离 -- 注释行，语句内勿写分号。
-- ============================================================

CREATE TABLE IF NOT EXISTS order_kitchen (
    kitchen_id SERIAL PRIMARY KEY,
    order_id INT NOT NULL,
    shop_id INT NOT NULL DEFAULT 0,
    kitchen_status SMALLINT NOT NULL DEFAULT 0,
    urge_count INT NOT NULL DEFAULT 0,
    last_urge_time INT NOT NULL DEFAULT 0,
    urge_unread SMALLINT NOT NULL DEFAULT 0,
    start_time INT NOT NULL DEFAULT 0,
    done_time INT NOT NULL DEFAULT 0,
    add_time INT NOT NULL DEFAULT 0,
    update_time INT NOT NULL DEFAULT 0
);

CREATE UNIQUE INDEX IF NOT EXISTS uk_order_kitchen_order ON order_kitchen (order_id);

CREATE INDEX IF NOT EXISTS idx_order_kitchen_shop_status ON order_kitchen (shop_id, kitchen_status);

CREATE INDEX IF NOT EXISTS idx_order_kitchen_urge_unread ON order_kitchen (urge_unread, last_urge_time);

CREATE TABLE IF NOT EXISTS order_kitchen_urge (
    urge_id SERIAL PRIMARY KEY,
    order_id INT NOT NULL,
    shop_id INT NOT NULL DEFAULT 0,
    user_id INT NOT NULL DEFAULT 0,
    add_time INT NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_kitchen_urge_order_user ON order_kitchen_urge (order_id, user_id, add_time);
