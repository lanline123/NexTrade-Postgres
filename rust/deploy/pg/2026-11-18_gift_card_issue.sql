-- 2026-11-18: 礼品卡补齐 P1（发放闭环）+ 标准对齐改造（D2–D6/D10）。
-- 内容：发放批次/明细两表 + 消费流水表（结算抵扣/返还写入）+
-- gift_card_groups 兑换模式两列 + gift_cards 储值卡余额列与存量回填。
-- 幂等：CREATE TABLE / ADD COLUMN 均 IF NOT EXISTS，回填 UPDATE 带条件可重复执行。
-- 回滚：DROP TABLE gift_card_issue_log / gift_card_issue_batch / gift_card_usage_log;
--       ALTER TABLE gift_card_groups DROP COLUMN exchange_mode, DROP COLUMN select_count;
--       ALTER TABLE gift_cards DROP COLUMN balance;
-- 注：cards_per_user 列保留兼容历史数据；标准对齐 D3 定案一人一卡，
--     发放链路恒写 1，不再按该列放大发放数量。
BEGIN;

-- ------------------------------------------------------------
-- 发放批次（照 coupon_issue_batch 扩卡组维度）
-- ------------------------------------------------------------
CREATE TABLE IF NOT EXISTS gift_card_issue_batch (
    id             bigserial PRIMARY KEY,
    card_groups_id integer      NOT NULL DEFAULT 0,
    shop_id        integer      NOT NULL DEFAULT 0,
    issue_type     varchar(16)  NOT NULL DEFAULT 'USER',
    target_desc    jsonb        NOT NULL DEFAULT '{}'::jsonb,
    cards_per_user integer      NOT NULL DEFAULT 1,
    target_count   integer      NOT NULL DEFAULT 0,
    success_count  integer      NOT NULL DEFAULT 0,
    fail_count     integer      NOT NULL DEFAULT 0,
    admin_id       integer      NOT NULL DEFAULT 0,
    remark         varchar(255) NOT NULL DEFAULT '',
    add_time       bigint       NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_gift_card_issue_batch_group ON gift_card_issue_batch (card_groups_id, id DESC);
CREATE INDEX IF NOT EXISTS idx_gift_card_issue_batch_time ON gift_card_issue_batch (add_time DESC);

-- ------------------------------------------------------------
-- 发放明细（只记 gift_cards_id，卡号/卡密不进流水）
-- ------------------------------------------------------------
CREATE TABLE IF NOT EXISTS gift_card_issue_log (
    id             bigserial PRIMARY KEY,
    batch_id       bigint  NOT NULL DEFAULT 0,
    card_groups_id integer NOT NULL DEFAULT 0,
    gift_cards_id  integer NOT NULL DEFAULT 0,
    user_id        integer NOT NULL DEFAULT 0,
    issue_time     bigint  NOT NULL DEFAULT 0,
    CONSTRAINT uq_gift_card_issue_log_batch_card UNIQUE (batch_id, gift_cards_id)
);

CREATE INDEX IF NOT EXISTS idx_gift_card_issue_log_batch ON gift_card_issue_log (batch_id, id);
CREATE INDEX IF NOT EXISTS idx_gift_card_issue_log_user ON gift_card_issue_log (user_id, id DESC);
CREATE INDEX IF NOT EXISTS idx_gift_card_issue_log_card ON gift_card_issue_log (gift_cards_id);

-- ------------------------------------------------------------
-- 消费流水（结算抵扣/返还的权威流水）
-- type: 1 结算抵扣 / 2 订单取消或退款返还；amount 带符号。
-- 标准对齐 D4 定案：储值卡「兑换入会员余额」已下线，本表不设 type=3。
-- ------------------------------------------------------------
CREATE TABLE IF NOT EXISTS gift_card_usage_log (
    id             bigserial PRIMARY KEY,
    gift_cards_id  integer        NOT NULL DEFAULT 0,
    user_id        integer        NOT NULL DEFAULT 0,
    order_id       integer        NOT NULL DEFAULT 0,
    type           smallint       NOT NULL DEFAULT 0,
    amount         numeric(10, 2) NOT NULL DEFAULT 0,
    balance_before numeric(10, 2),
    balance_after  numeric(10, 2),
    add_time       bigint         NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_gift_card_usage_log_card ON gift_card_usage_log (gift_cards_id, id);
CREATE INDEX IF NOT EXISTS idx_gift_card_usage_log_user ON gift_card_usage_log (user_id, id DESC);

-- ------------------------------------------------------------
-- 卡组：兑换模式（仅兑换卡有意义；useCard 固定/任选核销已落地，D6）
-- exchange_mode: 1 固定套餐 / 2 多件任选；select_count: 任选必选件数
-- ------------------------------------------------------------
ALTER TABLE gift_card_groups ADD COLUMN IF NOT EXISTS exchange_mode integer NOT NULL DEFAULT 1;
ALTER TABLE gift_card_groups ADD COLUMN IF NOT EXISTS select_count integer NOT NULL DEFAULT 0;

-- ------------------------------------------------------------
-- 礼品卡：储值卡剩余余额（兑换卡恒 NULL）
-- 存量回填：储值卡组（group_type=1）且未使用（status=0）的卡 balance = 卡组面额；
-- 已使用卡 balance = 0；兑换卡保持 NULL。带 balance IS NULL 条件，可重复执行。
-- ------------------------------------------------------------
ALTER TABLE gift_cards ADD COLUMN IF NOT EXISTS balance numeric(10, 2);

UPDATE gift_cards c
SET balance = g.face_amount
FROM gift_card_groups g
WHERE c.card_groups_id = g.card_groups_id
  AND g.group_type = 1
  AND c.status = 0
  AND c.balance IS NULL;

UPDATE gift_cards c
SET balance = 0
FROM gift_card_groups g
WHERE c.card_groups_id = g.card_groups_id
  AND g.group_type = 1
  AND c.status <> 0
  AND c.balance IS NULL;

COMMIT;
