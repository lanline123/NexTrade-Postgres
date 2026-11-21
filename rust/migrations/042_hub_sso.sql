-- 平台联邦（hub）三期①：登录互认（授权码模式，设计 §6.3）。
-- 设计：~/workspace/federation-phase3/DESIGN.md §1；上位设计
-- ~/workspace/hub-federation-design/proposal.md §6.3。
-- 幂等：全部 IF NOT EXISTS，可重复执行。生产 auto_migrate=false，
-- 部署时先执行本 SQL 再发后端（服务层对新表直查，缺表即报错）。
BEGIN;

-- 中枢侧授权请求：成员后端经上行钥（member.auth）登记，成员本人（会员 JWT）
-- 在中枢同意/拒绝；同意后生成一次性授权码（只存哈希，5 分钟过期）。
-- return_url 的白名单 = 连接登记的成员地址（link.peer_base_url 前缀），不另设登记面。
-- state 为成员生成的防 CSRF 随机串（非凭据）：OAuth 标准要求授权方原样回带，
-- 故中枢存明文以便同意后回跳时回显；成员侧只存其哈希（hub_sso_state）。
CREATE TABLE IF NOT EXISTS hub_sso_request (
    id               serial PRIMARY KEY,
    link_id          integer      NOT NULL DEFAULT 0,  -- 申请方连接（类型③，中枢视角 role=1）
    state            varchar(128) NOT NULL DEFAULT '', -- 成员生成 state（同意后回跳原样回带）
    return_url       varchar(500) NOT NULL DEFAULT '', -- 同意后浏览器回跳地址（成员站回调页）
    status           smallint     NOT NULL DEFAULT 0,  -- 0 待同意 / 1 已同意 / 2 已拒绝 / 3 已过期
    hub_user_id      integer      NOT NULL DEFAULT 0,  -- 同意的中枢本地会员 user_id
    global_member_id varchar(40)  NOT NULL DEFAULT '', -- 同意时解析/新签的全局 ID
    code_hash        varchar(64)  NOT NULL DEFAULT '', -- 授权码 SHA-256（明文只在同意响应出现一次）
    code_expire_time integer      NOT NULL DEFAULT 0,  -- 授权码过期时间（同意后 5 分钟）
    code_used        smallint     NOT NULL DEFAULT 0,  -- 0 未用 / 1 已兑换（一次性）
    add_time         integer      NOT NULL DEFAULT 0,
    update_time      integer      NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS idx_hub_sso_request_link ON hub_sso_request (link_id, add_time);

-- 成员侧发起记录：发起时落 state 哈希，回调时按 state 找回连接与后续落号结果。
CREATE TABLE IF NOT EXISTS hub_sso_state (
    id            serial PRIMARY KEY,
    link_id       integer     NOT NULL DEFAULT 0, -- 目标中枢连接（成员视角 role=2）
    state_hash    varchar(64) NOT NULL DEFAULT '',
    status        smallint    NOT NULL DEFAULT 0, -- 0 待回调 / 1 已完成
    local_user_id integer     NOT NULL DEFAULT 0, -- 回调落成的本地会员 user_id
    add_time      integer     NOT NULL DEFAULT 0,
    update_time   integer     NOT NULL DEFAULT 0,
    UNIQUE (state_hash)
);

COMMIT;
