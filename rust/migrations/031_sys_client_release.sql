-- admin 自助热更新 P4（小程序 / App 更新提醒，我方新增，用户 2026-11-21 22:21 拍板开工）。
-- 设计：~/workspace/hot-update-design/REPORT.md §5.4 与 §6-P4。
-- 幂等：CREATE TABLE IF NOT EXISTS，可重复执行。
-- 生产 auto_migrate=false，部署时先执行本 SQL 再发后端（表缺失时仓储层降级为空集，
-- 登记页可达不 500；旧二进制不读本表，前后兼容）。
-- 编号说明：027 为热更新 P1/P2 的 sys_schema_migration/sys_update_job，本迁移顺延 028。
-- 若主聊合并时重排编号，改文件名即可（迁移登记只认文件名，代码无引用）。
BEGIN;

-- 客户端已发布版本登记（人工登记口径，设计 §9-8 推荐项）：
-- 一行 = 一次「某渠道已针对某产品版本完成发版」的登记。提醒消警口径：
-- 最新 Release 的清单标志（compat.requiresMiniProgramUpdate / requiresAppUpdate）
-- 为真且该渠道不存在 product_version >= 最新产品版本的登记 → 提醒持续存在；
-- 登记后自动消警（派生态，不存提醒行，无陈旧告警）。登记只增不改，纠错走删除后重登。
CREATE TABLE IF NOT EXISTS sys_client_release (
    id              bigserial PRIMARY KEY,
    channel         varchar(32)  NOT NULL DEFAULT '',  -- mini_program 小程序 / app App
    product_version varchar(32)  NOT NULL DEFAULT '',  -- 本次客户端发版对应的产品版本（x.y.z）
    client_version  varchar(64)  NOT NULL DEFAULT '',  -- 已发布客户端版本（小程序版本号 / App 版本号）
    note            varchar(500) NOT NULL DEFAULT '',  -- 备注（如提审单号、发布渠道）
    operator_id     bigint       NOT NULL DEFAULT 0,
    operator_name   varchar(64)  NOT NULL DEFAULT '',
    created_at      bigint       NOT NULL DEFAULT 0    -- 登记时刻（Unix 秒）
);
CREATE INDEX IF NOT EXISTS idx_sys_client_release_channel ON sys_client_release (channel, created_at DESC);

COMMIT;
