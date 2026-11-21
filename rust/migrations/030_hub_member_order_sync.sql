-- 平台联邦（hub）二期：会员同步 + 订单上行汇总（迁移 028，本线暂定号，合并时主聊统一改序）。
-- 设计：~/workspace/hub-federation-design/proposal.md §6（会员面：映射/匹配/冲突/资料同步）
-- 与 §7.1（订单头级上行汇总）。生产开关启用不在本迁移内（modules.member/order 仍默认关）。
-- 幂等：全部 IF NOT EXISTS，可重复执行。生产 auto_migrate=false，部署时先执行本 SQL 再发后端。
BEGIN;

-- 中枢侧会员映射（设计 §6.1 hub_member_link）：一行 = 一个全局会员在一个成员实例上的映射。
-- 同一 global_member_id 可有多行（每连接一行）；手机号只存哈希 + AES-GCM 密文（与出站钥同主密钥）。
CREATE TABLE IF NOT EXISTS hub_member_link (
    id                 serial PRIMARY KEY,
    global_member_id   varchar(40)  NOT NULL,           -- gm_<uuid>，中枢签发、终身不变
    link_id            integer      NOT NULL DEFAULT 0, -- 来源连接（类型③）
    remote_member_id   integer      NOT NULL DEFAULT 0, -- 成员侧本地 user_id
    mobile_hash        varchar(64)  NOT NULL DEFAULT '',-- SHA-256(手机号)，主匹配键
    mobile_enc         text         NOT NULL DEFAULT '',-- 手机号 AES-GCM 密文（CRM 展示解密后脱敏）
    nickname           varchar(100) NOT NULL DEFAULT '',
    avatar             varchar(500) NOT NULL DEFAULT '',
    birthday           varchar(16)  NOT NULL DEFAULT '',
    email              varchar(100) NOT NULL DEFAULT '',
    real_name          varchar(50)  NOT NULL DEFAULT '',-- 仅作冲突检测信号，不作主数据下发
    profile_updated_at integer      NOT NULL DEFAULT 0, -- 远端资料版本戳（后写优先比较用）
    status             smallint     NOT NULL DEFAULT 1, -- 1 已关联 / 3 已解绑
    add_time           integer      NOT NULL DEFAULT 0,
    update_time        integer      NOT NULL DEFAULT 0,
    UNIQUE (link_id, remote_member_id)
);
CREATE INDEX IF NOT EXISTS idx_hub_member_link_mobile ON hub_member_link (mobile_hash);
CREATE INDEX IF NOT EXISTS idx_hub_member_link_global ON hub_member_link (global_member_id);

-- 成员侧会员映射（设计 §6.1 hub_member_map）：本地 user_id ↔ 全局 ID。
CREATE TABLE IF NOT EXISTS hub_member_map (
    id                 serial PRIMARY KEY,
    link_id            integer     NOT NULL DEFAULT 0,
    local_user_id      integer     NOT NULL DEFAULT 0,
    global_member_id   varchar(40) NOT NULL DEFAULT '',
    profile_updated_at integer     NOT NULL DEFAULT 0,  -- 最近一次成功同步的资料版本戳
    add_time           integer     NOT NULL DEFAULT 0,
    update_time        integer     NOT NULL DEFAULT 0,
    UNIQUE (link_id, local_user_id)
);
CREATE INDEX IF NOT EXISTS idx_hub_member_map_global ON hub_member_map (global_member_id);

-- 中枢侧会员冲突队列（设计 §6.2）：弱信号永不自动合并，一律进队列人工裁决。
-- 行内带上报快照，裁决「关联/独立建号」时据此落 hub_member_link，不必回源再取。
CREATE TABLE IF NOT EXISTS hub_member_conflict (
    id                        serial PRIMARY KEY,
    link_id                   integer     NOT NULL DEFAULT 0,
    remote_member_id          integer     NOT NULL DEFAULT 0,
    mobile_hash               varchar(64) NOT NULL DEFAULT '',
    mobile_enc                text        NOT NULL DEFAULT '',
    nickname                  varchar(100) NOT NULL DEFAULT '',
    avatar                    varchar(500) NOT NULL DEFAULT '',
    birthday                  varchar(16)  NOT NULL DEFAULT '',
    email                     varchar(100) NOT NULL DEFAULT '',
    real_name                 varchar(50)  NOT NULL DEFAULT '',
    profile_updated_at        integer      NOT NULL DEFAULT 0,
    candidate_global_member_id varchar(40) NOT NULL DEFAULT '', -- 冲突指向的既有全局 ID（可空）
    conflict_type             varchar(32)  NOT NULL DEFAULT '', -- mobile_multi_remote / mobile_claimed / identity_mismatch / unverified_mobile
    status                    smallint     NOT NULL DEFAULT 0,  -- 0 待处理 / 1 已关联 / 2 独立建号 / 3 忽略
    add_time                  integer      NOT NULL DEFAULT 0,
    update_time               integer      NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS idx_hub_member_conflict_status ON hub_member_conflict (status, link_id);

-- 中枢侧订单上行汇总（设计 §7.1）：只存头级汇总，不存收货地址/收件人/明细行。
CREATE TABLE IF NOT EXISTS hub_order_summary (
    id                   serial PRIMARY KEY,
    link_id              integer       NOT NULL DEFAULT 0,
    remote_instance_uuid varchar(64)   NOT NULL DEFAULT '',
    remote_order_sn      varchar(64)   NOT NULL DEFAULT '',
    remote_shop_id       integer       NOT NULL DEFAULT 0,
    remote_shop_name     varchar(100)  NOT NULL DEFAULT '',
    remote_user_id       integer       NOT NULL DEFAULT 0,
    global_member_id     varchar(40)   NOT NULL DEFAULT '', -- 已关联买家的全局 ID，未关联为空
    order_amount         numeric(12,2) NOT NULL DEFAULT 0,
    paid_amount          numeric(12,2) NOT NULL DEFAULT 0,
    order_status         integer       NOT NULL DEFAULT 0,
    pay_status           integer       NOT NULL DEFAULT 0,
    item_count           integer       NOT NULL DEFAULT 0,  -- 件数（数量合计）
    order_time           integer       NOT NULL DEFAULT 0,
    pay_time             integer       NOT NULL DEFAULT 0,
    status_version       bigint        NOT NULL DEFAULT 0,  -- 远端事件时间戳，旧版本不覆盖新版本
    add_time             integer       NOT NULL DEFAULT 0,
    update_time          integer       NOT NULL DEFAULT 0,
    UNIQUE (link_id, remote_order_sn)
);
CREATE INDEX IF NOT EXISTS idx_hub_order_summary_link_time ON hub_order_summary (link_id, order_time);

-- 成员侧发送游标（手动同步的增量位点；行在成员侧即「我的连接」行，中枢侧不用）。
ALTER TABLE hub_link ADD COLUMN IF NOT EXISTS member_sync_cursor integer NOT NULL DEFAULT 0;
ALTER TABLE hub_link ADD COLUMN IF NOT EXISTS order_sync_cursor  integer NOT NULL DEFAULT 0;
ALTER TABLE hub_link ADD COLUMN IF NOT EXISTS order_sync_time    integer NOT NULL DEFAULT 0;

COMMIT;
