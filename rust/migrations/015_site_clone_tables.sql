-- 站点搬家（site_clone）P1 建表：搬家密钥 + 拉取审计 + 导入前快照 + 导入任务 + 去重映射。
-- 设计：~/workspace/site-clone-design/REPORT.md 第四/五/六节 + DOMAINS.md 附册四。
-- 出处：我方新增（原版 tigshop 无整站/跨站复制功能）。
-- 幂等：全部 IF NOT EXISTS，可重复执行。生产 auto_migrate=false，部署时先执行本 SQL 再发后端。
-- 编号说明：仓库现有最大编号为 011（011_min_buy_count）；并行线已分别占用 012（礼品卡适用范围）、013（积分兑换优惠券）、014（送礼物订单流），本迁移顺延 015。若主聊合并时重排编号，改文件名即可（迁移执行器只认文件名，代码无引用）。
BEGIN;

CREATE TABLE IF NOT EXISTS site_clone_key (
    key_id          serial PRIMARY KEY,
    key_code        varchar(40)  NOT NULL,              -- 公开标识 mv_ + 32 hex
    key_name        varchar(100) NOT NULL DEFAULT '',
    secret_hash     varchar(64)  NOT NULL DEFAULT '',   -- SHA-256(secret)，明文只在生成/重置时显示一次
    scopes          varchar(200) NOT NULL DEFAULT '',   -- 授权域逗号分隔：products,settings（P1 仅此两域）
    source_shop_id  integer      NOT NULL DEFAULT 0,    -- 分享源店铺（0 = 平台自营）
    expire_time     integer      NOT NULL DEFAULT 0,    -- 过期时间（秒级时间戳，0 = 长期）
    max_calls       integer      NOT NULL DEFAULT 0,    -- 拉取总次数上限（0 = 不限）
    used_calls      integer      NOT NULL DEFAULT 0,    -- 已用拉取次数
    status          smallint     NOT NULL DEFAULT 1,    -- 1 启用 / 0 停用 / -1 已吊销
    last_used_time  integer      NOT NULL DEFAULT 0,
    created_by      integer      NOT NULL DEFAULT 0,    -- 创建管理员 id
    created_by_name varchar(100) NOT NULL DEFAULT '',
    add_time        integer      NOT NULL DEFAULT 0,
    update_time     integer      NOT NULL DEFAULT 0
);
CREATE UNIQUE INDEX IF NOT EXISTS uk_site_clone_key_code ON site_clone_key (key_code);
CREATE INDEX IF NOT EXISTS idx_site_clone_key_status ON site_clone_key (status);

CREATE TABLE IF NOT EXISTS site_clone_call_log (
    log_id      serial PRIMARY KEY,
    key_id      integer      NOT NULL DEFAULT 0,
    key_code    varchar(40)  NOT NULL DEFAULT '',
    endpoint    varchar(120) NOT NULL DEFAULT '',       -- /clone/v1/manifest 等
    domain      varchar(40)  NOT NULL DEFAULT '',       -- manifest / catalog / products / settings
    http_status integer      NOT NULL DEFAULT 0,
    result_count integer     NOT NULL DEFAULT 0,        -- 本次返回条数
    client_ip   varchar(64)  NOT NULL DEFAULT '',
    duration_ms integer      NOT NULL DEFAULT 0,
    add_time    integer      NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS idx_site_clone_call_log_key ON site_clone_call_log (key_id, log_id DESC);
CREATE INDEX IF NOT EXISTS idx_site_clone_call_log_time ON site_clone_call_log (add_time DESC);

CREATE TABLE IF NOT EXISTS site_clone_snapshot (
    snapshot_id     serial PRIMARY KEY,
    job_id          integer      NOT NULL DEFAULT 0,    -- 关联导入任务
    operator_id     integer      NOT NULL DEFAULT 0,
    operator_name   varchar(100) NOT NULL DEFAULT '',
    source_label    varchar(200) NOT NULL DEFAULT '',   -- 来源站名/地址（展示用）
    fingerprint     varchar(64)  NOT NULL DEFAULT '',   -- 来源口令指纹 SHA-256(keyId+secret)
    payload         text         NOT NULL DEFAULT '',   -- 快照 JSON：{groups:{key:原值}, blocks:{code:原值或null}}
    group_keys      varchar(1000) NOT NULL DEFAULT '',  -- 本快照覆盖的设置组 key（逗号分隔，列表展示用）
    status          smallint     NOT NULL DEFAULT 1,    -- 1 有效 / 2 已回滚
    rollback_time   integer      NOT NULL DEFAULT 0,
    add_time        integer      NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS idx_site_clone_snapshot_time ON site_clone_snapshot (add_time DESC);

CREATE TABLE IF NOT EXISTS site_clone_import_job (
    job_id          serial PRIMARY KEY,
    fingerprint     varchar(64)  NOT NULL DEFAULT '',   -- 来源口令指纹
    source_origin   varchar(255) NOT NULL DEFAULT '',   -- 来源站地址
    source_label    varchar(200) NOT NULL DEFAULT '',   -- 来源站名（manifest 回填）
    target_shop_id  integer      NOT NULL DEFAULT 0,    -- 承接店铺
    domains         varchar(200) NOT NULL DEFAULT '',   -- 本次勾选域（逗号分隔）
    options         text         NOT NULL DEFAULT '',   -- 执行选项 JSON（设置组勾选/风险组/状态口径等）
    status          varchar(20)  NOT NULL DEFAULT 'running', -- running / success / partial / failed
    snapshot_id     integer      NOT NULL DEFAULT 0,    -- 本任务的导入前快照（无设置域时为 0）
    counts          text         NOT NULL DEFAULT '',   -- 逐域计数 JSON：{created, skipped, failed, ...}
    failures        text         NOT NULL DEFAULT '',   -- 失败明细 JSON 数组：[{domain, sourceId, name, error}]
    operator_id     integer      NOT NULL DEFAULT 0,
    operator_name   varchar(100) NOT NULL DEFAULT '',
    undone          smallint     NOT NULL DEFAULT 0,    -- 1 已撤销本次导入（新建商品已删）
    add_time        integer      NOT NULL DEFAULT 0,
    finish_time     integer      NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS idx_site_clone_job_time ON site_clone_import_job (add_time DESC);
CREATE INDEX IF NOT EXISTS idx_site_clone_job_fp ON site_clone_import_job (fingerprint);

CREATE TABLE IF NOT EXISTS site_clone_import_map (
    map_id      serial PRIMARY KEY,
    fingerprint varchar(64)  NOT NULL DEFAULT '',       -- 来源口令指纹
    domain      varchar(40)  NOT NULL DEFAULT '',       -- category / brand / shipping_type / shipping_tpl / product / service
    source_id   integer      NOT NULL DEFAULT 0,        -- 源站对象 id（运费模板等用源 id；按名对齐的结构域同样记录）
    source_key  varchar(300) NOT NULL DEFAULT '',       -- 源对象自然键（分类路径 / 品牌名 / 模板名 / 商品 SN 等，审计与展示用）
    target_id   integer      NOT NULL DEFAULT 0,        -- 本站对象 id
    job_id      integer      NOT NULL DEFAULT 0,        -- 首次导入的任务 id
    add_time    integer      NOT NULL DEFAULT 0
);
CREATE UNIQUE INDEX IF NOT EXISTS uk_site_clone_map ON site_clone_import_map (fingerprint, domain, source_id);
CREATE INDEX IF NOT EXISTS idx_site_clone_map_job ON site_clone_import_map (job_id);

COMMIT;
