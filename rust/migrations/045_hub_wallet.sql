-- 平台联邦（hub）三期④：中央账本（设计 §6.5）。
-- 设计：~/workspace/federation-phase3/DESIGN.md §4；上位设计
-- ~/workspace/hub-federation-design/proposal.md §6.5。
-- 资金红线：中枢是唯一记账方，只记账不划拨真实资金；差错只冲正不改史；
-- 余额/积分永不扣成负数。幂等键 = (link_id, biz_sn)。
-- 幂等：全部 IF NOT EXISTS，可重复执行。生产 auto_migrate=false，
-- 部署时先执行本 SQL 再发后端。
BEGIN;

-- 中枢侧会员联邦账户（一人一行，全局 ID 为主键）。
CREATE TABLE IF NOT EXISTS hub_wallet_account (
    global_member_id varchar(40)   PRIMARY KEY,
    points_balance   bigint        NOT NULL DEFAULT 0,
    money_balance    numeric(12,2) NOT NULL DEFAULT 0,
    frozen_money     numeric(12,2) NOT NULL DEFAULT 0,
    version          integer       NOT NULL DEFAULT 0,  -- 乐观锁，每次记账 +1
    status           smallint      NOT NULL DEFAULT 1,  -- 1 正常 / 2 冻结停用
    add_time         integer       NOT NULL DEFAULT 0,
    update_time      integer       NOT NULL DEFAULT 0
);

-- 中枢侧账本行（只追加，冲正以反向行 + 原行置已冲正表达，永不改史删史）。
CREATE TABLE IF NOT EXISTS hub_wallet_line (
    id               serial PRIMARY KEY,
    global_member_id varchar(40)   NOT NULL DEFAULT '',
    link_id          integer       NOT NULL DEFAULT 0,  -- 记账来源连接；中枢管理动作 = 0
    currency         varchar(10)   NOT NULL DEFAULT 'money', -- points / money
    biz_type         varchar(24)   NOT NULL DEFAULT '',
    -- points_grant / points_spend / money_credit / money_debit / money_freeze /
    -- money_confirm / money_release / reverse / merge_in
    amount           numeric(14,2) NOT NULL DEFAULT 0,  -- 带符号（积分以整数值存）
    balance_after    numeric(14,2) NOT NULL DEFAULT 0,  -- 记账后对应币种余额（中枢生成）
    frozen_after     numeric(12,2) NOT NULL DEFAULT 0,  -- 记账后冻结余额（money 类）
    biz_sn           varchar(64)   NOT NULL DEFAULT '', -- 来源业务单号（幂等键）
    ref_biz_sn       varchar(64)   NOT NULL DEFAULT '', -- 冻结/冲正引用单号
    ref_line_id      integer       NOT NULL DEFAULT 0,
    status           smallint      NOT NULL DEFAULT 1,  -- 1 已记账 / 2 已被冲正
    remark           varchar(255)  NOT NULL DEFAULT '',
    add_time         integer       NOT NULL DEFAULT 0,
    UNIQUE (link_id, biz_sn)
);
CREATE INDEX IF NOT EXISTS idx_hub_wallet_line_member ON hub_wallet_line (global_member_id, add_time);
CREATE INDEX IF NOT EXISTS idx_hub_wallet_line_freeze ON hub_wallet_line (global_member_id, biz_type, status);

-- 成员侧账本镜像（中枢回执的本地留痕，一连接一业务单号一行）。
CREATE TABLE IF NOT EXISTS hub_wallet_mirror (
    id               serial PRIMARY KEY,
    link_id          integer       NOT NULL DEFAULT 0,
    local_user_id    integer       NOT NULL DEFAULT 0,
    global_member_id varchar(40)   NOT NULL DEFAULT '',
    biz_sn           varchar(64)   NOT NULL DEFAULT '',
    currency         varchar(10)   NOT NULL DEFAULT 'money',
    biz_type         varchar(24)   NOT NULL DEFAULT '',
    amount           numeric(14,2) NOT NULL DEFAULT 0,
    balance_after    numeric(14,2) NOT NULL DEFAULT 0,
    add_time         integer       NOT NULL DEFAULT 0,
    UNIQUE (link_id, biz_sn)
);
CREATE INDEX IF NOT EXISTS idx_hub_wallet_mirror_user ON hub_wallet_mirror (local_user_id, add_time);

-- 成员侧并账状态（一人一行）：merged=1 后本地余额支付被守卫拒绝，
-- 联邦资产以中枢账本为准；本地列不清零（清零是资金动作，运营按快照核对后手工处理）。
CREATE TABLE IF NOT EXISTS hub_wallet_local_state (
    local_user_id    integer       PRIMARY KEY,
    link_id          integer       NOT NULL DEFAULT 0,
    global_member_id varchar(40)   NOT NULL DEFAULT '',
    merged           smallint      NOT NULL DEFAULT 0,
    points_merged    bigint        NOT NULL DEFAULT 0,
    money_merged     numeric(12,2) NOT NULL DEFAULT 0,
    add_time         integer       NOT NULL DEFAULT 0,
    update_time      integer       NOT NULL DEFAULT 0
);

COMMIT;
