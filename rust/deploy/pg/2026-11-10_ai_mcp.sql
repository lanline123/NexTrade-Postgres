-- 2026-11-10: AI 运营 MCP（商品域 + 装修域）审计与审批表。
-- 设置项存 config.biz_code = 'aiMcpSettings'（JSON），无需建表。
BEGIN;

CREATE TABLE IF NOT EXISTS ai_mcp_log (
    log_id      serial PRIMARY KEY,
    tool        varchar(64) NOT NULL,
    args        text,
    status      varchar(16) NOT NULL,
    error       varchar(500),
    duration_ms integer NOT NULL DEFAULT 0,
    add_time    integer NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_ai_mcp_log_add_time ON ai_mcp_log (add_time DESC);

CREATE TABLE IF NOT EXISTS ai_mcp_approval (
    approval_id  serial PRIMARY KEY,
    tool         varchar(64) NOT NULL,
    args         text NOT NULL,
    status       varchar(16) NOT NULL DEFAULT 'pending',
    add_time     integer NOT NULL,
    approve_time integer,
    approve_by   integer,
    result       text
);
CREATE INDEX IF NOT EXISTS idx_ai_mcp_approval_status ON ai_mcp_approval (status, approval_id DESC);

COMMIT;
