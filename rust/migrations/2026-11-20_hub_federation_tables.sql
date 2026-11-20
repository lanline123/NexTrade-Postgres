-- 平台联邦（hub）P1 建表（migrations 版，与 deploy/pg 同内容）：通用凭证 + 连接 + 配对码 + 调用日志 + 商品聚合映射。
-- 设计：~/workspace/hub-federation-design/proposal.md §2/§3/§4/§5/§8。
-- 幂等：全部 IF NOT EXISTS，可重复执行。生产 auto_migrate=false，部署时先执行本 SQL 再发后端。
BEGIN;

CREATE TABLE IF NOT EXISTS hub_credential (
    credential_id   serial PRIMARY KEY,
    conn_type       smallint NOT NULL DEFAULT 1,        -- 1 单向供货 / 2 单向接入 / 3 全联邦
    link_id         integer  NOT NULL DEFAULT 0,        -- 类型③所属连接；①②为 0
    direction       smallint NOT NULL DEFAULT 1,        -- 1 入站（我签发，只存哈希）/ 2 出站（我持有，存加密形态）
    peer_base_url   varchar(255) NOT NULL DEFAULT '',   -- 仅出站独立凭证（类型②）用
    cred_name       varchar(100) NOT NULL DEFAULT '',
    access_key      varchar(32)  NOT NULL,
    secret_hash     varchar(64)  NOT NULL DEFAULT '',   -- 入站钥：SHA-256(secret)
    secret_enc      text         NOT NULL DEFAULT '',   -- 出站钥：AES-GCM 密文（base64）
    scopes          bigint       NOT NULL DEFAULT 0,    -- 能力位集，见 core SCOPE_*
    scope_all_shops smallint     NOT NULL DEFAULT 1,    -- 1 全部店铺 / 0 按 hub_credential_shop
    scope_mode      smallint     NOT NULL DEFAULT 1,    -- 商品范围：1 全部 / 2 指定商品 / 3 指定分类（含后代）
    include_inquiry smallint     NOT NULL DEFAULT 0,
    price_rule_type varchar(16)  NOT NULL DEFAULT 'none',
    price_rule_value numeric(12,2) NOT NULL DEFAULT 0,
    status          smallint     NOT NULL DEFAULT 1,    -- 1 启用 / 0 停用
    expire_time     integer      NOT NULL DEFAULT 0,    -- 0 永不过期
    rate_limit_per_min integer   NOT NULL DEFAULT 600,
    daily_quota     bigint       NOT NULL DEFAULT 0,    -- 0 不限
    ip_whitelist    text         NOT NULL DEFAULT '',   -- P1 落列，默认不强制
    last_used_time  integer      NOT NULL DEFAULT 0,
    call_count      bigint       NOT NULL DEFAULT 0,
    add_time        integer      NOT NULL DEFAULT 0,
    update_time     integer      NOT NULL DEFAULT 0
);
CREATE UNIQUE INDEX IF NOT EXISTS uk_hub_credential_access_key ON hub_credential (access_key);
CREATE INDEX IF NOT EXISTS idx_hub_credential_link ON hub_credential (link_id);
CREATE INDEX IF NOT EXISTS idx_hub_credential_status ON hub_credential (status);

CREATE TABLE IF NOT EXISTS hub_credential_shop (
    id            serial PRIMARY KEY,
    credential_id integer NOT NULL,
    shop_id       integer NOT NULL,
    UNIQUE (credential_id, shop_id)
);
CREATE INDEX IF NOT EXISTS idx_hub_cred_shop_cred ON hub_credential_shop (credential_id);

CREATE TABLE IF NOT EXISTS hub_credential_product (
    id            serial PRIMARY KEY,
    credential_id integer NOT NULL,
    product_id    integer NOT NULL,
    UNIQUE (credential_id, product_id)
);
CREATE INDEX IF NOT EXISTS idx_hub_cred_product_cred ON hub_credential_product (credential_id);

CREATE TABLE IF NOT EXISTS hub_credential_category (
    id            serial PRIMARY KEY,
    credential_id integer NOT NULL,
    category_id   integer NOT NULL,
    UNIQUE (credential_id, category_id)
);
CREATE INDEX IF NOT EXISTS idx_hub_cred_category_cred ON hub_credential_category (credential_id);

CREATE TABLE IF NOT EXISTS hub_credential_price_override (
    id            serial PRIMARY KEY,
    credential_id integer NOT NULL,
    product_id    integer NOT NULL,
    rule_type     varchar(16) NOT NULL DEFAULT 'none',
    rule_value    numeric(12,2) NOT NULL DEFAULT 0,
    add_time      integer NOT NULL DEFAULT 0,
    update_time   integer NOT NULL DEFAULT 0,
    UNIQUE (credential_id, product_id)
);
CREATE INDEX IF NOT EXISTS idx_hub_cred_override_cred ON hub_credential_price_override (credential_id);

CREATE TABLE IF NOT EXISTS hub_link (
    link_id            serial PRIMARY KEY,
    link_type          smallint NOT NULL DEFAULT 3,     -- 恒 3（全联邦）；①②以凭证为管理单位不入此表
    role               smallint NOT NULL DEFAULT 1,     -- 1 我是中枢 / 2 我是成员
    peer_link_id       integer  NOT NULL DEFAULT 0,     -- 对端库中的连接 id（仅成员侧用）
    peer_instance_uuid varchar(64)  NOT NULL DEFAULT '',
    peer_name          varchar(100) NOT NULL DEFAULT '',
    peer_base_url      varchar(255) NOT NULL DEFAULT '',
    peer_version       varchar(32)  NOT NULL DEFAULT '',
    status             smallint NOT NULL DEFAULT 1,     -- 1 待审批 / 2 已激活 / 3 已停用 / 4 已拒绝 / 5 已过期
    approved_modules   bigint   NOT NULL DEFAULT 0,     -- 位集：1 商品 / 2 会员 / 4 订单
    up_credential_id   integer  NOT NULL DEFAULT 0,     -- 成员调中枢的钥（在本库的行）
    down_credential_id integer  NOT NULL DEFAULT 0,     -- 中枢调成员的钥（在本库的行）
    partner_user_id    integer  NOT NULL DEFAULT 0,     -- 中枢侧伙伴账户（分销员 user_id），P1 只落结构
    settle_mode        varchar(16) NOT NULL DEFAULT '', -- prepaid / periodic / commission（P3 执行）
    settle_cycle       varchar(16) NOT NULL DEFAULT '', -- t_plus_N / weekly / monthly
    fund_channel       varchar(16) NOT NULL DEFAULT '', -- own / channel_split
    rate_source        varchar(16) NOT NULL DEFAULT 'level', -- level / link_override
    rate_override      numeric(12,2) NOT NULL DEFAULT 0,
    claim_token_hash   varchar(64) NOT NULL DEFAULT '', -- 中枢侧存哈希；成员侧同值
    claim_token_enc    text        NOT NULL DEFAULT '', -- 仅成员侧：轮询/领取用的加密形态
    claim_expire_time  integer     NOT NULL DEFAULT 0,
    reject_reason      varchar(255) NOT NULL DEFAULT '',
    last_heartbeat_time integer    NOT NULL DEFAULT 0,
    heartbeat_payload  text        NOT NULL DEFAULT '',
    add_time           integer     NOT NULL DEFAULT 0,
    update_time        integer     NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS idx_hub_link_status ON hub_link (status);
CREATE INDEX IF NOT EXISTS idx_hub_link_peer_uuid ON hub_link (peer_instance_uuid);

CREATE TABLE IF NOT EXISTS hub_pairing_code (
    id           serial PRIMARY KEY,
    code_hash    varchar(64)  NOT NULL,                 -- 只存 SHA-256
    expect_name  varchar(100) NOT NULL DEFAULT '',
    status       smallint NOT NULL DEFAULT 0,           -- 0 未用 / 1 已用 / 2 作废
    fail_count   integer  NOT NULL DEFAULT 0,
    expire_time  integer  NOT NULL DEFAULT 0,
    used_link_id integer  NOT NULL DEFAULT 0,
    add_time     integer  NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS idx_hub_pairing_hash ON hub_pairing_code (code_hash);

CREATE TABLE IF NOT EXISTS hub_call_log (
    log_id        serial PRIMARY KEY,
    link_id       integer NOT NULL DEFAULT 0,
    credential_id integer NOT NULL DEFAULT 0,
    access_key    varchar(32)  NOT NULL DEFAULT '',
    endpoint      varchar(128) NOT NULL DEFAULT '',
    scope         varchar(32)  NOT NULL DEFAULT '',
    direction     smallint NOT NULL DEFAULT 0,
    http_status   integer  NOT NULL DEFAULT 0,
    result_count  integer  NOT NULL DEFAULT 0,
    client_ip     varchar(64) NOT NULL DEFAULT '',
    duration_ms   integer  NOT NULL DEFAULT 0,
    add_time      integer  NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS idx_hub_call_log_cred_time ON hub_call_log (credential_id, add_time);
CREATE INDEX IF NOT EXISTS idx_hub_call_log_link ON hub_call_log (link_id);

CREATE TABLE IF NOT EXISTS hub_product_map (
    map_id               serial PRIMARY KEY,
    link_id              integer NOT NULL DEFAULT 0,
    credential_id        integer NOT NULL DEFAULT 0,    -- 拉取所用出站凭证（类型②为独立凭证、link_id=0）
    remote_instance_uuid varchar(64)  NOT NULL DEFAULT '',
    remote_shop_id       integer NOT NULL DEFAULT 0,
    remote_shop_name     varchar(100) NOT NULL DEFAULT '',
    remote_product_id    integer NOT NULL,
    hub_product_id       integer NOT NULL DEFAULT 0,    -- 落本地商品后回填（P1 聚合先落映射+快照）
    product_name         varchar(255) NOT NULL DEFAULT '',
    pic_url              varchar(500) NOT NULL DEFAULT '',
    price                numeric(12,2) NOT NULL DEFAULT 0,
    market_price         numeric(12,2) NOT NULL DEFAULT 0,
    stock                integer NOT NULL DEFAULT 0,
    content_hash         varchar(64) NOT NULL DEFAULT '',
    snapshot             text NOT NULL DEFAULT '',      -- 远端商品详情 JSON 快照
    sync_state           smallint NOT NULL DEFAULT 1,   -- 1 正常 / 2 源下架 / 3 源不可见 / 4 已断开
    last_sync_time       integer NOT NULL DEFAULT 0,
    add_time             integer NOT NULL DEFAULT 0,
    update_time          integer NOT NULL DEFAULT 0,
    UNIQUE (link_id, credential_id, remote_product_id)
);
CREATE INDEX IF NOT EXISTS idx_hub_product_map_link ON hub_product_map (link_id);

COMMIT;
