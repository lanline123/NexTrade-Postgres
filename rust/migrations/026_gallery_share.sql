-- 平台与门店相册互通（论坛 B 类缓做项 #35 帖 687807 余子项，我方新增）。
-- 设计：~/workspace/forum-b28/album-platform-shop-sync/REPORT.md 第二节。
-- 幂等：ADD COLUMN IF NOT EXISTS / CREATE TABLE IF NOT EXISTS，可重复执行。
-- 生产 auto_migrate=false，部署时先执行本 SQL 再发后端（新二进制的实体含
-- is_shared 列，旧二进制不读该列，前后兼容）。
-- 编号说明：仓库现有最大编号 015（015_site_clone_tables），本迁移顺延 016。
-- 若主聊合并时重排编号，改文件名即可（迁移执行器只认文件名，代码无引用）。
BEGIN;

-- 1) 下发标记：平台分组（shop_id = 0）is_shared = 1 时对全部门店可见（子树继承）。
ALTER TABLE gallery       ADD COLUMN IF NOT EXISTS is_shared integer NOT NULL DEFAULT 0;
ALTER TABLE gallery_video ADD COLUMN IF NOT EXISTS is_shared integer NOT NULL DEFAULT 0;

-- 2) 存量回填（保现行口径）：现行线上口径为「平台素材所有店铺可读」，存量平台
--    分组在迁移时一律置为已下发，避免上线当天门店突然看不到平台素材；
--    迁移之后新建的平台分组默认未下发，由平台管理员按需下发/收回。
UPDATE gallery       SET is_shared = 1 WHERE shop_id = 0 AND is_shared = 0;
UPDATE gallery_video SET is_shared = 1 WHERE shop_id = 0 AND is_shared = 0;

-- 3) 门店素材上报平台审核表：门店提交本店素材 → 平台审核 → 通过后复制入平台库。
CREATE TABLE IF NOT EXISTS gallery_share_submit (
    id                serial PRIMARY KEY,
    shop_id           integer      NOT NULL DEFAULT 0,   -- 上报门店
    media_type        varchar(10)  NOT NULL DEFAULT 'image', -- image / video
    source_gallery_id integer      NOT NULL DEFAULT 0,   -- 来源分组（快照）
    source_item_id    integer      NOT NULL DEFAULT 0,   -- 来源素材 id（gallery_pic.pic_id / gallery_video_info.id）
    item_name         varchar(500) NOT NULL DEFAULT '',  -- 素材名快照
    item_url          text         NOT NULL DEFAULT '',  -- 素材地址快照
    item_thumb        text         NOT NULL DEFAULT '',  -- 缩略图/封面快照
    status            integer      NOT NULL DEFAULT 0,   -- 0 待审核 / 1 已通过 / 2 已驳回
    submit_admin_id   integer      NOT NULL DEFAULT 0,
    submit_time       integer      NOT NULL DEFAULT 0,
    review_admin_id   integer      NOT NULL DEFAULT 0,
    review_time       integer      NOT NULL DEFAULT 0,
    review_remark     varchar(500) NOT NULL DEFAULT '',
    target_gallery_id integer      NOT NULL DEFAULT 0,   -- 通过时落入的平台分组
    result_item_id    integer      NOT NULL DEFAULT 0,   -- 通过后生成的平台素材 id
    add_time          integer      NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS idx_gallery_share_submit_shop   ON gallery_share_submit (shop_id, id DESC);
CREATE INDEX IF NOT EXISTS idx_gallery_share_submit_status ON gallery_share_submit (status, id DESC);
-- 同一素材同时只允许一条待审核记录（重复上报在 DB 层兜底，服务层转友好文案）
CREATE UNIQUE INDEX IF NOT EXISTS uk_gallery_share_submit_pending
    ON gallery_share_submit (shop_id, media_type, source_item_id) WHERE status = 0;

COMMIT;
