-- 2026-11-19: admin AI 助手（我方新增）数据表 + 权限播种。
-- 类型口径：禁用 SMALLINT；主键/外键/枚举/标志一律 INT；积分/token/时间戳一律 BIGINT（epoch 秒）。
-- P1 使用：ai_channel / ai_channel_key / ai_model / ai_conversation / ai_message / ai_action_audit / ai_usage。
-- 预留（P2/P3 建表不用）：ai_credit_account / ai_credit_log / ai_credit_order / ai_price_rule / ai_image_task。
BEGIN;

CREATE TABLE IF NOT EXISTS ai_channel (
    channel_id  serial PRIMARY KEY,
    shop_id     integer NOT NULL DEFAULT 0,          -- owner：0=平台，>0=商家本店
    name        varchar(100) NOT NULL,
    kind        integer NOT NULL DEFAULT 2,          -- 1 官方 / 2 OpenAI 兼容 / 3 蜂鸟 / 4 火山
    base_url    varchar(500) NOT NULL DEFAULT '',
    is_enabled  integer NOT NULL DEFAULT 1,
    is_default  integer NOT NULL DEFAULT 0,
    sort        integer NOT NULL DEFAULT 0,
    create_by   integer NOT NULL DEFAULT 0,
    create_time bigint NOT NULL DEFAULT 0,
    update_time bigint NOT NULL DEFAULT 0,
    is_del      integer NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS idx_ai_channel_owner ON ai_channel (shop_id, is_del, sort);

CREATE TABLE IF NOT EXISTS ai_channel_key (
    key_id       serial PRIMARY KEY,
    channel_id   integer NOT NULL,
    key_cipher   text NOT NULL DEFAULT '',           -- AES-256-GCM 密文（Base64）
    key_nonce    varchar(64) NOT NULL DEFAULT '',    -- GCM nonce（Base64）
    key_mask     varchar(20) NOT NULL DEFAULT '',    -- ****后4位
    extra_cipher text NOT NULL DEFAULT '',           -- 第二凭据密文（蜂鸟 ProjectId / 火山 SK）
    extra_nonce  varchar(64) NOT NULL DEFAULT '',
    extra_mask   varchar(20) NOT NULL DEFAULT '',
    status       integer NOT NULL DEFAULT 1,         -- 1 现行 / 0 已轮换
    create_time  bigint NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS idx_ai_channel_key_channel ON ai_channel_key (channel_id, status);

CREATE TABLE IF NOT EXISTS ai_model (
    model_id       serial PRIMARY KEY,
    channel_id     integer NOT NULL,
    model_code     varchar(200) NOT NULL,
    display_name   varchar(200) NOT NULL DEFAULT '',
    capabilities   varchar(100) NOT NULL DEFAULT 'chat',  -- 逗号集：chat,vision,image_gen,translate,inpaint
    context_tokens integer NOT NULL DEFAULT 0,
    is_enabled     integer NOT NULL DEFAULT 1,
    sort           integer NOT NULL DEFAULT 0,
    is_del         integer NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS idx_ai_model_channel ON ai_model (channel_id, is_del, sort);

CREATE TABLE IF NOT EXISTS ai_conversation (
    conv_id     serial PRIMARY KEY,
    shop_id     integer NOT NULL DEFAULT 0,          -- 会话租户（创建时固化）
    admin_id    integer NOT NULL DEFAULT 0,          -- 创建人
    title       varchar(200) NOT NULL DEFAULT '',
    channel_id  integer NOT NULL DEFAULT 0,
    model_code  varchar(200) NOT NULL DEFAULT '',
    status      integer NOT NULL DEFAULT 1,          -- 1 正常 / 0 删除
    create_time bigint NOT NULL DEFAULT 0,
    update_time bigint NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS idx_ai_conv_owner ON ai_conversation (shop_id, admin_id, update_time DESC);

CREATE TABLE IF NOT EXISTS ai_message (
    msg_id           bigserial PRIMARY KEY,
    conv_id          integer NOT NULL,
    role             integer NOT NULL,               -- 1 user / 2 assistant / 3 tool / 4 system-summary
    content          text NOT NULL DEFAULT '',
    tool_calls       jsonb,                          -- assistant 行：[{id,name,args}]；tool 行：{callId,name,args}
    tool_name        varchar(100) NOT NULL DEFAULT '',
    tool_status      integer NOT NULL DEFAULT 0,     -- 0 无 / 1 待确认 / 2 已执行 / 3 已拒绝 / 4 失败
    prompt_tokens    integer NOT NULL DEFAULT 0,
    completion_tokens integer NOT NULL DEFAULT 0,
    credits_charged  bigint NOT NULL DEFAULT 0,
    create_time      bigint NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS idx_ai_msg_conv ON ai_message (conv_id, msg_id);

CREATE TABLE IF NOT EXISTS ai_action_audit (
    audit_id    bigserial PRIMARY KEY,
    shop_id     integer NOT NULL DEFAULT 0,
    admin_id    integer NOT NULL DEFAULT 0,
    conv_id     integer NOT NULL DEFAULT 0,
    tool        varchar(100) NOT NULL DEFAULT '',
    target_type varchar(50) NOT NULL DEFAULT '',
    target_id   integer NOT NULL DEFAULT 0,
    args_digest text NOT NULL DEFAULT '',            -- 脱敏摘要（不含密钥/正文）
    result      integer NOT NULL DEFAULT 1,          -- 1 成功 / 2 拒绝-越权 / 3 拒绝-未注册 / 4 失败 / 5 待确认取消
    create_time bigint NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS idx_ai_audit_shop ON ai_action_audit (shop_id, create_time DESC);
CREATE INDEX IF NOT EXISTS idx_ai_audit_result ON ai_action_audit (result, create_time DESC);

CREATE TABLE IF NOT EXISTS ai_usage (
    usage_id          bigserial PRIMARY KEY,
    shop_id           integer NOT NULL DEFAULT 0,
    admin_id          integer NOT NULL DEFAULT 0,
    conv_id           integer NOT NULL DEFAULT 0,
    channel_id        integer NOT NULL DEFAULT 0,
    model_code        varchar(200) NOT NULL DEFAULT '',
    kind              integer NOT NULL DEFAULT 1,    -- 1 对话 / 2 图片 / 3 翻译 / 4 消除
    prompt_tokens     integer NOT NULL DEFAULT 0,
    completion_tokens integer NOT NULL DEFAULT 0,
    image_count       integer NOT NULL DEFAULT 0,
    credits_charged   bigint NOT NULL DEFAULT 0,
    self_key          integer NOT NULL DEFAULT 1,    -- 1 第三方自有 key 不扣分（P1 恒 1）
    status            integer NOT NULL DEFAULT 1,    -- 1 成功 / 2 失败 / 3 部分
    error             varchar(500) NOT NULL DEFAULT '',
    create_time       bigint NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS idx_ai_usage_shop ON ai_usage (shop_id, create_time DESC);

-- ——— 以下为 P2/P3 预留表（本期只建表） ———

CREATE TABLE IF NOT EXISTS ai_credit_account (
    account_id      serial PRIMARY KEY,
    shop_id         integer NOT NULL UNIQUE,         -- 0=平台账户
    balance         bigint NOT NULL DEFAULT 0,
    total_recharged bigint NOT NULL DEFAULT 0,
    total_consumed  bigint NOT NULL DEFAULT 0,
    status          integer NOT NULL DEFAULT 1,      -- 1 正常 / 0 冻结
    update_time     bigint NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS ai_credit_log (
    log_id        bigserial PRIMARY KEY,
    account_id    integer NOT NULL DEFAULT 0,
    shop_id       integer NOT NULL DEFAULT 0,
    type          integer NOT NULL,                  -- 1 充值 / 2 消费 / 3 失败退回 / 4 平台调整 / 5 赠送
    delta         bigint NOT NULL DEFAULT 0,
    balance_after bigint NOT NULL DEFAULT 0,
    ref_type      varchar(30) NOT NULL DEFAULT '',
    ref_id        bigint NOT NULL DEFAULT 0,
    usage_id      bigint NOT NULL DEFAULT 0,
    remark        varchar(500) NOT NULL DEFAULT '',
    create_time   bigint NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS idx_ai_credit_log_shop ON ai_credit_log (shop_id, create_time DESC);

CREATE TABLE IF NOT EXISTS ai_credit_order (
    order_id       serial PRIMARY KEY,
    order_sn       varchar(40) NOT NULL UNIQUE,
    shop_id        integer NOT NULL DEFAULT 0,
    buyer_admin_id integer NOT NULL DEFAULT 0,
    amount         decimal(12,2) NOT NULL DEFAULT 0,
    credits        bigint NOT NULL DEFAULT 0,
    pay_type       varchar(30) NOT NULL DEFAULT '',
    pay_status     integer NOT NULL DEFAULT 0,       -- 0 待付 / 1 已付 / 2 已取消 / 3 已退款
    paylog_id      integer NOT NULL DEFAULT 0,
    pay_time       bigint NOT NULL DEFAULT 0,
    create_time    bigint NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS ai_price_rule (
    rule_id       serial PRIMARY KEY,
    model_code    varchar(200) NOT NULL,
    kind          integer NOT NULL DEFAULT 1,
    input_per_1k  decimal(12,4) NOT NULL DEFAULT 0,  -- 积分/千 token
    output_per_1k decimal(12,4) NOT NULL DEFAULT 0,
    image_price   bigint NOT NULL DEFAULT 0,         -- 积分/张
    is_enabled    integer NOT NULL DEFAULT 1,
    update_time   bigint NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS ai_image_task (
    task_id            serial PRIMARY KEY,
    shop_id            integer NOT NULL DEFAULT 0,
    admin_id           integer NOT NULL DEFAULT 0,
    conv_id            integer NOT NULL DEFAULT 0,
    channel_id         integer NOT NULL DEFAULT 0,
    model_code         varchar(200) NOT NULL DEFAULT '',
    prompt             text NOT NULL DEFAULT '',
    size               varchar(20) NOT NULL DEFAULT '',
    status             integer NOT NULL DEFAULT 0,   -- 0 生成中 / 1 成功 / 2 失败
    upstream_task_id   varchar(200) NOT NULL DEFAULT '',
    result_pic_id      integer NOT NULL DEFAULT 0,
    applied_product_id integer NOT NULL DEFAULT 0,
    applied_position   integer NOT NULL DEFAULT 0,   -- 1 主图 / 2 相册 / 3 详情
    usage_id           bigint NOT NULL DEFAULT 0,
    error              varchar(500) NOT NULL DEFAULT '',
    create_time        bigint NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS idx_ai_image_task_shop ON ai_image_task (shop_id, create_time DESC);

-- ——— 权限播种：AI 助手组三页（平台 admin + 商家 shop 双行，父级挂 MCP 管理组） ———

INSERT INTO authority (authority_sn, authority_name, parent_id, sort_order, is_show, child_auth, route_link, authority_ico, is_system, admin_type)
SELECT 'aiAssistantManage', 'AI 助手', COALESCE((SELECT authority_id FROM authority WHERE authority_sn = 'aiMcpManage' LIMIT 1), 0),
       10, 1, '[]', 'aiAssistant', '', 0, 'admin'
WHERE NOT EXISTS (SELECT 1 FROM authority WHERE authority_sn = 'aiAssistantManage' AND admin_type = 'admin');

INSERT INTO authority (authority_sn, authority_name, parent_id, sort_order, is_show, child_auth, route_link, authority_ico, is_system, admin_type)
SELECT 'aiAssistantManage', 'AI 助手', COALESCE((SELECT authority_id FROM authority WHERE authority_sn = 'aiMcpManage' LIMIT 1), 0),
       10, 1, '[]', 'aiAssistant', '', 0, 'shop'
WHERE NOT EXISTS (SELECT 1 FROM authority WHERE authority_sn = 'aiAssistantManage' AND admin_type = 'shop');

INSERT INTO authority (authority_sn, authority_name, parent_id, sort_order, is_show, child_auth, route_link, authority_ico, is_system, admin_type)
SELECT 'aiChannelsManage', '渠道配置', COALESCE((SELECT authority_id FROM authority WHERE authority_sn = 'aiMcpManage' LIMIT 1), 0),
       11, 1, '[]', 'aiChannels', '', 0, 'admin'
WHERE NOT EXISTS (SELECT 1 FROM authority WHERE authority_sn = 'aiChannelsManage' AND admin_type = 'admin');

INSERT INTO authority (authority_sn, authority_name, parent_id, sort_order, is_show, child_auth, route_link, authority_ico, is_system, admin_type)
SELECT 'aiChannelsManage', '渠道配置', COALESCE((SELECT authority_id FROM authority WHERE authority_sn = 'aiMcpManage' LIMIT 1), 0),
       11, 1, '[]', 'aiChannels', '', 0, 'shop'
WHERE NOT EXISTS (SELECT 1 FROM authority WHERE authority_sn = 'aiChannelsManage' AND admin_type = 'shop');

INSERT INTO authority (authority_sn, authority_name, parent_id, sort_order, is_show, child_auth, route_link, authority_ico, is_system, admin_type)
SELECT 'aiUsageManage', '用量记录', COALESCE((SELECT authority_id FROM authority WHERE authority_sn = 'aiMcpManage' LIMIT 1), 0),
       12, 1, '[]', 'aiUsage', '', 0, 'admin'
WHERE NOT EXISTS (SELECT 1 FROM authority WHERE authority_sn = 'aiUsageManage' AND admin_type = 'admin');

INSERT INTO authority (authority_sn, authority_name, parent_id, sort_order, is_show, child_auth, route_link, authority_ico, is_system, admin_type)
SELECT 'aiUsageManage', '用量记录', COALESCE((SELECT authority_id FROM authority WHERE authority_sn = 'aiMcpManage' LIMIT 1), 0),
       12, 1, '[]', 'aiUsage', '', 0, 'shop'
WHERE NOT EXISTS (SELECT 1 FROM authority WHERE authority_sn = 'aiUsageManage' AND admin_type = 'shop');

COMMIT;
