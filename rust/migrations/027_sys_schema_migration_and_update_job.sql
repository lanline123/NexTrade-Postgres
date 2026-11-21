-- admin 自助热更新 P1/P2（我方新增，用户 2026-11-21 22:13 拍板开工）。
-- 设计：~/workspace/hot-update-design/REPORT.md §4.1（登记表）与 §5（执行与审计）。
-- 幂等：CREATE TABLE IF NOT EXISTS，可重复执行。
-- 生产 auto_migrate=false，部署时先执行本 SQL 再发后端（新后端的状态接口读这两张表；
-- 表缺失时服务层降级为「基线未就绪」不报错，旧二进制不读这两张表，前后兼容）。
-- 编号说明：仓库现有最大编号 026（026_gallery_share），本迁移顺延 027。
-- 若主聊合并时重排编号，改文件名即可（迁移登记只认文件名，代码无引用）。
BEGIN;

-- 1) 迁移登记表：唯一可信的「已执行迁移」事实源（替代靠探列/探表反推）。
--    不复用代码内 _tig_migrations（无校验和列、生产从未启用、语义被朴素执行器绑定）。
--    基线回填由 tools/hot-update/baseline.py 生成脚本以 applied_by='baseline' 补登记，
--    此后每次部署/热更新执行迁移后逐文件登记（含 sha256，文件被改即可发现）。
CREATE TABLE IF NOT EXISTS sys_schema_migration (
    filename        varchar(255) PRIMARY KEY,          -- 迁移文件名（如 027_xxx.sql）
    sha256          char(64)     NOT NULL DEFAULT '',  -- 执行时文件内容的 sha256
    product_version varchar(32)  NOT NULL DEFAULT '',  -- 随哪个产品版本执行
    applied_at      bigint       NOT NULL DEFAULT 0,   -- 执行时刻（Unix 秒）
    applied_by      varchar(64)  NOT NULL DEFAULT '',  -- 操作人用户名 / 'baseline' / 'deploy'
    job_id          varchar(64)  NOT NULL DEFAULT ''   -- 热更新 job id（基线/人工部署为空）
);

-- 2) 热更新任务审计表：一行 = 一次在 admin 发起的更新 job（状态机见设计 §5.2）。
--    manifest_snapshot 钉死 job 创建时的 release-manifest.json 全文——执行只认
--    快照与其中 sha256，禁止执行时再取 latest（设计安全红线第 1 条）。
CREATE TABLE IF NOT EXISTS sys_update_job (
    job_id            varchar(64) PRIMARY KEY,           -- uuid
    product_version   varchar(32)  NOT NULL DEFAULT '',  -- 目标产品版本
    manifest_sha256   char(64)     NOT NULL DEFAULT '',  -- manifest 文件本身的 sha256
    manifest_snapshot text         NOT NULL DEFAULT '',  -- release-manifest.json 全文
    surfaces          varchar(255) NOT NULL DEFAULT '',  -- 选中面，逗号分隔（admin,pc,h5）
    status            varchar(32)  NOT NULL DEFAULT 'queued',
    -- status: queued/downloading/verifying/backing_up/swapping/health_checking/
    --         done/failed/rolling_back/rolled_back/rollback_failed
    stage             varchar(64)  NOT NULL DEFAULT '',  -- updater 回写的当前阶段细节
    message           text         NOT NULL DEFAULT '',  -- 状态说明/失败原因（不含任何凭据）
    operator_id       bigint       NOT NULL DEFAULT 0,
    operator_name     varchar(64)  NOT NULL DEFAULT '',
    destructive_ack   boolean      NOT NULL DEFAULT false, -- 破坏性迁移二次确认位（审计留痕）
    created_at        bigint       NOT NULL DEFAULT 0,
    updated_at        bigint       NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS idx_sys_update_job_created ON sys_update_job (created_at DESC);
CREATE INDEX IF NOT EXISTS idx_sys_update_job_status  ON sys_update_job (status, created_at DESC);

COMMIT;
