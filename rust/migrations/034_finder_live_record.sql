-- 视频号直播记录落库表（finderLiveRecord 后端补齐）。
-- 出处：接口本身原版有（Java ApiWechatLiveController#getFinderLiveRecord /
-- 管理端 WechatLiveController#getFinderLiveRecord，均只代理微信「视频号
-- 助手」开放接口 getfinderliverecordlist、不落库）；落库与落库查询为
-- 我方新增。设计：~/workspace/finder-live-record/REPORT.md。
-- 幂等：CREATE TABLE / INDEX IF NOT EXISTS，可重复执行。
-- 生产 auto_migrate=false，部署时先执行本 SQL 再发后端（旧二进制不读
-- 本表，前后兼容）。
-- 编号说明：按主聊合并口径取 028（027 预留并行线）。若合并时重排编号，
-- 改文件名即可（迁移执行器只认文件名，代码无引用）。
BEGIN;

-- 每次拉取到的当前直播记录整批落本表：export_id 唯一，is_current 标记
-- 最近一次拉取是否仍在当前列表（拉取刷新口径见仓储 replace_current）。
CREATE TABLE IF NOT EXISTS finder_live_record (
    id              serial PRIMARY KEY,
    export_id       varchar(128) NOT NULL DEFAULT '',  -- 微信直播 id（export_id），唯一键
    description     text         NOT NULL DEFAULT '',  -- 直播描述
    cover_url       text         NOT NULL DEFAULT '',  -- 直播封面
    nickname        varchar(255) NOT NULL DEFAULT '',  -- 开播视频号昵称
    head_url        text         NOT NULL DEFAULT '',  -- 开播视频号头像
    is_current      integer      NOT NULL DEFAULT 0,   -- 1 = 最近一次拉取仍在当前直播列表中
    first_seen_time integer      NOT NULL DEFAULT 0,   -- 首次拉取到（秒级时间戳）
    last_seen_time  integer      NOT NULL DEFAULT 0,   -- 最近一次拉取到（秒级时间戳）
    add_time        integer      NOT NULL DEFAULT 0,
    update_time     integer      NOT NULL DEFAULT 0
);
CREATE UNIQUE INDEX IF NOT EXISTS uk_finder_live_record_export
    ON finder_live_record (export_id);
CREATE INDEX IF NOT EXISTS idx_finder_live_record_current
    ON finder_live_record (is_current, last_seen_time DESC);

COMMIT;
