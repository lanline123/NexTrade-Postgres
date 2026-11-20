-- ============================================================
-- 009_aftersale_exchange.sql
-- 售后「换货」（我方新增，论坛帖 807883，用户 2026-11-21 拍板等价换 SKU 口径）：
-- aftersales.aftersale_type 新增取值 3 = 换货（原版枚举只有 1 退货/退款、
-- 2 仅退款；注意 aftersale_type 列注释把 2 写成换货是原版注释错误，
-- 2 恒为仅退款，一切按枚举口径）。
-- 本迁移只加列，不改既有数据：
-- 1) aftersales_item.exchange_sku_id：换货目标 SKU（同 SKU 补发时等于原 SKU；
--    无 SKU 商品为 NULL）。仅 type=3 换货单写入，其余类型恒 NULL。
-- 2) aftersales_exchange_delivery：换货重发物流侧表（一售后单至多一行，
--    aftersale_id 主键）。商家确认收货后「换货重发」时写入，
--    买家端/admin 详情据此展示重发物流；独立侧表避免改动 aftersales
--    主单的物流字段语义（logistics_name/tracking_no 是买家回寄物流）。
-- 部署顺序：本迁移必须先于新二进制换版执行（新版实体含 exchange_sku_id 列，
-- 旧版对多出的列/表不敏感，反序安全、正序必需）。
-- 注意：迁移执行器按分号逐条执行且会剥离 -- 注释行，语句内勿写分号。
-- ============================================================

ALTER TABLE aftersales_item ADD COLUMN IF NOT EXISTS exchange_sku_id INTEGER;

CREATE TABLE IF NOT EXISTS aftersales_exchange_delivery (
    aftersale_id INTEGER PRIMARY KEY,
    logistics_name VARCHAR(100) NOT NULL DEFAULT '',
    tracking_no VARCHAR(100) NOT NULL DEFAULT '',
    ship_time INTEGER NOT NULL DEFAULT 0,
    add_time INTEGER NOT NULL DEFAULT 0
);
