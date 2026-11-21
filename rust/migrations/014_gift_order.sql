-- ============================================================
-- 014_gift_order.sql
-- 送礼物订单流（论坛帖 197742，我方新增，用户 2026-11-21 拍板
-- 开工）：买家结算时选「送礼物」，付款后订单进入待领取，好友
-- 登录后填自己的收货地址领取，订单再落入正常履约链；超时未
-- 领取自动原路退款。礼物关系不侵入订单状态枚举，独立落
-- order_gift 一行（挂在买家下单的根订单上，跨店拆单时子单
-- 同步领取/落地址/退款）。
-- gift_status：0 待付款 1 待领取 2 已领取 3 已关闭（退款）
-- 4 未付款取消。领取 token 只存 SHA-256 摘要，明文仅在生成/
-- 重新生成时一次性返回买家。
-- 部署顺序：本迁移必须先于新二进制换版执行（新版会读写
-- order_gift 表，旧版对新表不敏感，反序安全、正序必需）。
-- 注意：迁移执行器按分号逐条执行且会剥离 -- 注释行，语句内勿写分号。
-- ============================================================

CREATE TABLE IF NOT EXISTS order_gift (
    gift_id SERIAL PRIMARY KEY,
    order_id INT NOT NULL,
    order_sn VARCHAR(64) NOT NULL DEFAULT '',
    buyer_user_id INT NOT NULL DEFAULT 0,
    gift_status SMALLINT NOT NULL DEFAULT 0,
    gift_message VARCHAR(200) NOT NULL DEFAULT '',
    token_hash CHAR(64) NOT NULL DEFAULT '',
    expire_time INT NOT NULL DEFAULT 0,
    claim_user_id INT NOT NULL DEFAULT 0,
    claim_time INT NOT NULL DEFAULT 0,
    close_reason VARCHAR(255) NOT NULL DEFAULT '',
    add_time INT NOT NULL DEFAULT 0,
    update_time INT NOT NULL DEFAULT 0
);

CREATE UNIQUE INDEX IF NOT EXISTS uk_order_gift_order ON order_gift (order_id);

CREATE INDEX IF NOT EXISTS idx_order_gift_token ON order_gift (token_hash);

CREATE INDEX IF NOT EXISTS idx_order_gift_expire ON order_gift (gift_status, expire_time);

CREATE INDEX IF NOT EXISTS idx_order_gift_claimer ON order_gift (claim_user_id);
