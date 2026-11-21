-- 平台联邦（hub）三期⑤：跨站结算（设计 §11）。
-- 设计：~/workspace/federation-phase3/DESIGN.md §5；上位设计
-- ~/workspace/hub-federation-design/proposal.md §11。
-- 资金红线：只记账、只生成结算单，不做真实跨站资金划拨；结清只是登记。
-- 主体口径（§11.6 硬约束）：账目以 link_id / credential_id 为主体键，
-- partner_user_id 仅为连接伙伴账户的快照字段，绝不用 salesman_id 代替连接。
-- 幂等：全部 IF NOT EXISTS，可重复执行。生产 auto_migrate=false，
-- 部署时先执行本 SQL 再发后端。
BEGIN;

-- 预付账户（主体 = 连接或凭证；供货预付模式扣减对象）。
CREATE TABLE IF NOT EXISTS hub_settle_account (
    id           serial PRIMARY KEY,
    subject_type varchar(16)   NOT NULL DEFAULT 'link', -- link / credential
    subject_id   integer       NOT NULL DEFAULT 0,
    balance      numeric(12,2) NOT NULL DEFAULT 0,      -- 可扣成负（负额在账户页告警）
    warn_line    numeric(12,2) NOT NULL DEFAULT 0,      -- 预警线：余额低于此值告警
    status       smallint      NOT NULL DEFAULT 1,
    add_time     integer       NOT NULL DEFAULT 0,
    update_time  integer       NOT NULL DEFAULT 0,
    UNIQUE (subject_type, subject_id)
);

-- 账目行（分账/回传/扣减/结转的统一账目；只记账目，不执行渠道分账）。
CREATE TABLE IF NOT EXISTS hub_settle_line (
    id              serial PRIMARY KEY,
    link_id         integer       NOT NULL DEFAULT 0,
    credential_id   integer       NOT NULL DEFAULT 0,   -- 类型①②主体时填
    partner_user_id integer       NOT NULL DEFAULT 0,   -- 连接伙伴账户快照（非主体键）
    source_type     varchar(20)   NOT NULL DEFAULT '',
    -- xorder 跨店分账 / xorder_refund 跨店退款冲抵 / supply_return 供货成交回传 /
    -- supply_refund 供货退款冲抵 / prepaid_deduct 预付扣减 / carry_in 结转入
    remote_order_sn varchar(64)   NOT NULL DEFAULT '',
    item_name       varchar(255)  NOT NULL DEFAULT '',  -- 商品快照
    quantity        integer       NOT NULL DEFAULT 0,
    deal_amount     numeric(12,2) NOT NULL DEFAULT 0,   -- 成交额（退款行为负）
    supply_amount   numeric(12,2) NOT NULL DEFAULT 0,   -- 供货价快照/预付扣减额
    rate            numeric(8,4)  NOT NULL DEFAULT 0,   -- 费率快照（百分率数值，如 5 = 5%）
    rate_pending    smallint      NOT NULL DEFAULT 0,   -- 1 = 费率未定（拦结算单生成）
    self_due        numeric(12,2) NOT NULL DEFAULT 0,   -- 记账方（中枢）应得
    peer_due        numeric(12,2) NOT NULL DEFAULT 0,   -- 对端应得
    statement_id    integer       NOT NULL DEFAULT 0,   -- 0 = 未入单
    line_status     smallint      NOT NULL DEFAULT 1,   -- 1 正常 / 2 差异挂起 / 3 已冲正
    biz_sn          varchar(80)   NOT NULL DEFAULT '',  -- 幂等键
    remark          varchar(255)  NOT NULL DEFAULT '',
    add_time        integer       NOT NULL DEFAULT 0,
    update_time     integer       NOT NULL DEFAULT 0,
    UNIQUE (link_id, credential_id, biz_sn)
);
CREATE INDEX IF NOT EXISTS idx_hub_settle_line_stmt ON hub_settle_line (link_id, statement_id, line_status);
CREATE INDEX IF NOT EXISTS idx_hub_settle_line_time ON hub_settle_line (link_id, add_time);

-- 结算单（对账单；net = peer_total − self_total，正 = 记账方应付对端）。
CREATE TABLE IF NOT EXISTS hub_settle_statement (
    id              serial PRIMARY KEY,
    statement_sn    varchar(40)   NOT NULL DEFAULT '',
    link_id         integer       NOT NULL DEFAULT 0,
    credential_id   integer       NOT NULL DEFAULT 0,
    partner_user_id integer       NOT NULL DEFAULT 0,
    period_start    integer       NOT NULL DEFAULT 0,
    period_end      integer       NOT NULL DEFAULT 0,
    total_deal      numeric(12,2) NOT NULL DEFAULT 0,
    self_total      numeric(12,2) NOT NULL DEFAULT 0,
    peer_total      numeric(12,2) NOT NULL DEFAULT 0,
    net_amount      numeric(12,2) NOT NULL DEFAULT 0,
    line_count      integer       NOT NULL DEFAULT 0,
    diff_count      integer       NOT NULL DEFAULT 0,
    statement_type  smallint      NOT NULL DEFAULT 1,   -- 1 普通 / 2 轧差
    self_confirm    smallint      NOT NULL DEFAULT 0,
    peer_confirm    smallint      NOT NULL DEFAULT 0,
    exec_status     smallint      NOT NULL DEFAULT 0,   -- 0 待确认 / 1 已确认待结清 / 2 已结清 / 3 已结转下期
    carry_amount    numeric(12,2) NOT NULL DEFAULT 0,
    add_time        integer       NOT NULL DEFAULT 0,
    update_time     integer       NOT NULL DEFAULT 0,
    UNIQUE (statement_sn)
);
CREATE INDEX IF NOT EXISTS idx_hub_settle_statement_link ON hub_settle_statement (link_id, add_time);

COMMIT;
