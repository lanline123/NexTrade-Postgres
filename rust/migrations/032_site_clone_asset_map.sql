-- 站点搬家（site_clone）P2 建表：素材转存映射（源 URL → 本站 URL）。
-- 设计：~/workspace/site-clone-design/REPORT.md 第八节 P2 + DOMAINS.md 附册四。
-- 出处：我方新增（原版 tigshop 无整站/跨站复制功能）。
-- 幂等：IF NOT EXISTS，可重复执行。生产 auto_migrate=false，部署时先执行本 SQL 再发后端。
-- 编号说明：本迁移在 P2 暂存时取 028（026 已被相册互通占用、027 预留热更新线）；
-- 若主聊合并时重排编号，改文件名即可（迁移执行器只认文件名，代码无引用）。
BEGIN;

CREATE TABLE IF NOT EXISTS site_clone_asset_map (
    map_id        serial PRIMARY KEY,
    fingerprint   varchar(64)  NOT NULL DEFAULT '',   -- 来源口令指纹 SHA-256(keyId+secret)
    source_url    text         NOT NULL DEFAULT '',   -- 源站文件 URL（完整 http(s) 地址）
    local_url     text         NOT NULL DEFAULT '',   -- 转存后本站 URL（存储后端返回的访问地址）
    media_kind    varchar(20)  NOT NULL DEFAULT '',   -- pic / video / file（按扩展名粗分）
    target_pic_id integer      NOT NULL DEFAULT 0,    -- 素材库导入新建的 gallery_pic.pic_id（仅图片库条目）
    job_id        integer      NOT NULL DEFAULT 0,    -- 首次转存的导入任务 id
    add_time      integer      NOT NULL DEFAULT 0
);
CREATE UNIQUE INDEX IF NOT EXISTS uk_site_clone_asset_map
    ON site_clone_asset_map (fingerprint, source_url);
CREATE INDEX IF NOT EXISTS idx_site_clone_asset_map_job
    ON site_clone_asset_map (job_id);

COMMIT;
