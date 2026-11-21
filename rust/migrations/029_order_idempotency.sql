-- ============================================================
-- 029_order_idempotency.sql
-- 下单提交幂等键（我方新增，用户 2026-11-21 拍板开工）：买家在
-- 弱网重试、重复点击、客户端并发双发时，同一笔下单意图只生成
-- 一个订单。客户端为一次结算会话生成一个 UUID 随提交下发，
-- 服务端按下单用户 + 幂等键登记一行，与建单同事务写入（主键
-- 唯一约束仲裁并发双发），窗口内（24 小时）同键重复提交直接
-- 返回原单结果（响应快照），不再重复算价、扣库存、占用优惠。
-- 口径：未传键或键非法的请求不启用幂等，存量客户端行为逐字
-- 不变；响应快照不含送礼物 token 明文（order_gift 只存摘要的
-- 既有口径），重放命中礼物单时买家走订单详情重新生成分享。
-- 部署顺序：本迁移必须先于新二进制换版执行（新版会读写新表，
-- 旧版对新表不敏感，反序安全、正序必需）。
-- 注意：迁移执行器按分号逐条执行且会剥离 -- 注释行，语句内勿写分号。
-- ============================================================

CREATE TABLE IF NOT EXISTS order_idempotency (
    user_id INT NOT NULL,
    idem_key VARCHAR(64) NOT NULL,
    order_id INT,
    order_sn VARCHAR(64) NOT NULL DEFAULT '',
    response_json TEXT NOT NULL DEFAULT '',
    add_time INT NOT NULL DEFAULT 0,
    PRIMARY KEY (user_id, idem_key)
);

CREATE INDEX IF NOT EXISTS idx_order_idempotency_add_time ON order_idempotency (add_time);
