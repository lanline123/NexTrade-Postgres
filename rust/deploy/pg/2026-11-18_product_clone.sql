-- 2026-11-18: 商品复刻（跨平台商品搬家）任务表。
-- 双通道：api = 第三方数据接口（万邦 onebound），selfhost = 自建抓取边车。
-- attempts 留痕每次尝试（通道/结果/费用），cost_fen 为通道 A 累计费用（分，预估口径）。
-- 幂等：CREATE TABLE IF NOT EXISTS，可重复执行。
BEGIN;

CREATE TABLE IF NOT EXISTS product_clone_task (
    id            bigserial PRIMARY KEY,
    platform      varchar(16)  NOT NULL DEFAULT '',
    channel       varchar(16)  NOT NULL DEFAULT 'api',
    source_url    text         NOT NULL DEFAULT '',
    source_item_id varchar(64) NOT NULL DEFAULT '',
    status        varchar(16)  NOT NULL DEFAULT 'parsing',
    title         varchar(500) NOT NULL DEFAULT '',
    draft_json    jsonb,
    raw_snapshot  jsonb,
    attempts      jsonb        NOT NULL DEFAULT '[]'::jsonb,
    cost_fen      integer      NOT NULL DEFAULT 0,
    product_id    integer,
    error_msg     text         NOT NULL DEFAULT '',
    created_by    varchar(64)  NOT NULL DEFAULT '',
    create_time   bigint       NOT NULL DEFAULT 0,
    update_time   bigint       NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_product_clone_task_status ON product_clone_task (status, id DESC);
CREATE INDEX IF NOT EXISTS idx_product_clone_task_source ON product_clone_task (platform, source_item_id);

COMMIT;
