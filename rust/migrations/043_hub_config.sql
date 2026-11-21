-- 平台联邦（hub）三期②：配置分发（拉模式，设计 §8.3）。
-- 设计：~/workspace/federation-phase3/DESIGN.md §2；上位设计
-- ~/workspace/hub-federation-design/proposal.md §8.3。
-- 幂等：全部 IF NOT EXISTS，可重复执行。生产 auto_migrate=false，
-- 部署时先执行本 SQL 再发后端。
BEGIN;

-- 中枢侧配置包：version 为本实例单调递增序号（发布时取 max+1），
-- payload 只存规范化 JSON 文本，content_hash = SHA-256(payload 文本)。
-- 包签名不落库：下发时以连接下行钥密钥现场派生（见服务层）。
CREATE TABLE IF NOT EXISTS hub_config_package (
    id             serial PRIMARY KEY,
    package_type   varchar(32)  NOT NULL DEFAULT 'custom', -- announcement 公告 / activity 活动 / channel 频道配置 / custom 自定义
    title          varchar(200) NOT NULL DEFAULT '',
    version        integer      NOT NULL DEFAULT 0,        -- 本实例单调递增，下发/拉取按此比对
    payload        text         NOT NULL DEFAULT '{}',
    content_hash   varchar(64)  NOT NULL DEFAULT '',
    apply_mode     smallint     NOT NULL DEFAULT 1,        -- 1 手动确认 / 2 自动应用
    effective_from integer      NOT NULL DEFAULT 0,        -- 0 = 不限
    effective_to   integer      NOT NULL DEFAULT 0,        -- 0 = 不限
    status         smallint     NOT NULL DEFAULT 1,        -- 1 生效 / 0 已撤销
    add_time       integer      NOT NULL DEFAULT 0,
    update_time    integer      NOT NULL DEFAULT 0,
    UNIQUE (version)
);
CREATE INDEX IF NOT EXISTS idx_hub_config_package_status ON hub_config_package (status, version);

-- 中枢侧回执：成员应用/拒绝/失败后回报，一连接一包一行（upsert）。
CREATE TABLE IF NOT EXISTS hub_config_ack (
    id          serial PRIMARY KEY,
    link_id     integer     NOT NULL DEFAULT 0,
    package_id  integer     NOT NULL DEFAULT 0,
    version     integer     NOT NULL DEFAULT 0,
    ack_status  smallint    NOT NULL DEFAULT 0, -- 1 已应用 / 2 已拒绝 / 3 应用失败
    message     varchar(500) NOT NULL DEFAULT '',
    add_time    integer     NOT NULL DEFAULT 0,
    update_time integer     NOT NULL DEFAULT 0,
    UNIQUE (link_id, package_id)
);

-- 成员侧落地：拉取验签通过的包快照与应用状态（应用语义见 DESIGN §2.3：
-- announcement 类型回写本地平台公告，其余类型只落快照并如实记录）。
CREATE TABLE IF NOT EXISTS hub_config_apply (
    id            serial PRIMARY KEY,
    link_id       integer      NOT NULL DEFAULT 0, -- 来源中枢连接（成员视角 role=2）
    package_id    integer      NOT NULL DEFAULT 0, -- 中枢侧包 id
    version       integer      NOT NULL DEFAULT 0,
    package_type  varchar(32)  NOT NULL DEFAULT 'custom',
    title         varchar(200) NOT NULL DEFAULT '',
    payload       text         NOT NULL DEFAULT '{}',
    content_hash  varchar(64)  NOT NULL DEFAULT '',
    apply_mode    smallint     NOT NULL DEFAULT 1,
    apply_status  smallint     NOT NULL DEFAULT 0, -- 0 待处理 / 1 已应用 / 2 已拒绝 / 3 失败
    apply_message varchar(500) NOT NULL DEFAULT '',
    add_time      integer      NOT NULL DEFAULT 0,
    update_time   integer      NOT NULL DEFAULT 0,
    UNIQUE (link_id, package_id)
);
CREATE INDEX IF NOT EXISTS idx_hub_config_apply_link ON hub_config_apply (link_id, version);

COMMIT;
