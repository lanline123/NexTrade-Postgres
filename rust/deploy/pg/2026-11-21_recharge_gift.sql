-- 2026-11-21: 余额充值送实物礼品（论坛帖 717721，我方新增）。
--
-- 内容：
--   1) recharge_setting 增加赠品两列：gift_id 指向既有活动赠品池
--      product_gift.gift_id（0 = 不送实物），gift_quantity 为每单赠送数量。
--      与 discount_money（赠送金额）相互独立、可叠加；存量档位默认 0，
--      行为逐字不变。
--   2) user_recharge_order 增加 recharge_id：下单时快照命中的档位 id
--      （与 discount_money 快照同点），支付成功发放赠品时按快照取档，
--      避免档位事后编辑/删除影响已下单未确认的充值单。0 = 未命中档位
--      （自定义金额无匹配档 / 后台建单），不发赠品。
--   3) 新表 recharge_gift_record：充值赠品领取记录。充值单置「已支付」
--      时同事务生成一条（recharge_order_id 唯一，防重复发放）；
--      会员领取时扣减 product_gift 赠品池库存并生成 ¥0 履约订单
--      （order_type = 12）走商家正常发货链。
--      状态：0 待领取 / 1 已领取（已生成履约单）/ 2 已作废 / 3 异常（库存不足等）。
--
-- 幂等：ADD COLUMN / CREATE TABLE / CREATE INDEX 均 IF NOT EXISTS，可重复执行。
-- 回滚：DROP TABLE recharge_gift_record;
--       ALTER TABLE recharge_setting DROP COLUMN gift_id, DROP COLUMN gift_quantity;
--       ALTER TABLE user_recharge_order DROP COLUMN recharge_id;
BEGIN;

ALTER TABLE recharge_setting
    ADD COLUMN IF NOT EXISTS gift_id integer NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS gift_quantity integer NOT NULL DEFAULT 0;

ALTER TABLE user_recharge_order
    ADD COLUMN IF NOT EXISTS recharge_id integer NOT NULL DEFAULT 0;

CREATE TABLE IF NOT EXISTS recharge_gift_record (
    record_id         bigserial PRIMARY KEY,
    recharge_order_id integer      NOT NULL DEFAULT 0,
    recharge_id       integer      NOT NULL DEFAULT 0,
    user_id           integer      NOT NULL DEFAULT 0,
    gift_id           integer      NOT NULL DEFAULT 0,
    gift_name         varchar(200) NOT NULL DEFAULT '',
    product_id        integer      NOT NULL DEFAULT 0,
    sku_id            integer      NOT NULL DEFAULT 0,
    shop_id           integer      NOT NULL DEFAULT 0,
    quantity          integer      NOT NULL DEFAULT 1,
    status            integer      NOT NULL DEFAULT 0,
    order_id          integer      NOT NULL DEFAULT 0,
    address_id        integer      NOT NULL DEFAULT 0,
    consignee         varchar(100) NOT NULL DEFAULT '',
    mobile            varchar(50)  NOT NULL DEFAULT '',
    address_text      varchar(500) NOT NULL DEFAULT '',
    admin_note        varchar(255) NOT NULL DEFAULT '',
    add_time          integer      NOT NULL DEFAULT 0,
    claim_time        integer      NOT NULL DEFAULT 0,
    update_time       integer      NOT NULL DEFAULT 0,
    CONSTRAINT uq_recharge_gift_record_order UNIQUE (recharge_order_id)
);

CREATE INDEX IF NOT EXISTS idx_recharge_gift_record_user ON recharge_gift_record (user_id, record_id DESC);
CREATE INDEX IF NOT EXISTS idx_recharge_gift_record_status ON recharge_gift_record (status, record_id DESC);

COMMIT;
