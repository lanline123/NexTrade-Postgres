-- 抽奖「分享得次数」（论坛 B 类缓做项 #37，帖 717751）：活动分享奖励配置 + 分享奖励入账流水。
-- 出处：我方新增（原版抽奖次数来源为固定/配置口径，无分享得次数接线；B-EVAL 第 D 节第 37 条）。
-- 设计：分享奖励不新造次数账本——奖励次数以流水表为唯一事实源，info/draw 的有效上限 =
--       基础上限 + 已入账奖励次数（按日/累计分别汇总）；防刷口径见服务层
--       （事件幂等 + 每日/活动期分享次数上限 + 两次奖励最小间隔 60 秒）。
-- 幂等：ADD COLUMN / CREATE TABLE / INDEX 全部 IF NOT EXISTS，可重复执行。
-- 生产 auto_migrate=false，部署时先执行本 SQL 再发后端。
-- 编号说明：仓库现有最大编号为 015（015_site_clone_tables），本迁移顺延 016。
-- 若主聊合并时与并行线撞号，重排只需改文件名（迁移执行器只认文件名，代码无引用）。
BEGIN;

-- ————— 活动分享奖励配置（默认全关/零值，存量活动行为零变化） —————
ALTER TABLE lottery_activity ADD COLUMN IF NOT EXISTS share_reward_enabled integer NOT NULL DEFAULT 0;
ALTER TABLE lottery_activity ADD COLUMN IF NOT EXISTS share_reward_count integer NOT NULL DEFAULT 1;
ALTER TABLE lottery_activity ADD COLUMN IF NOT EXISTS share_reward_daily_limit integer NOT NULL DEFAULT 0;
ALTER TABLE lottery_activity ADD COLUMN IF NOT EXISTS share_reward_total_limit integer NOT NULL DEFAULT 0;

COMMENT ON COLUMN lottery_activity.share_reward_enabled IS '分享得次数开关：0 关闭 / 1 开启（我方新增）';
COMMENT ON COLUMN lottery_activity.share_reward_count IS '每次有效分享奖励的抽奖次数（我方新增）';
COMMENT ON COLUMN lottery_activity.share_reward_daily_limit IS '每人每日可获奖励的分享次数，0 = 不限制（我方新增）';
COMMENT ON COLUMN lottery_activity.share_reward_total_limit IS '每人活动期可获奖励的分享次数，0 = 不限制（我方新增）';

-- ————— 分享奖励入账流水（次数入账可查；event_id 幂等防重复发放） —————
CREATE TABLE IF NOT EXISTS lottery_share_reward_log (
    log_id       serial PRIMARY KEY,
    activity_id  integer      NOT NULL,               -- 抽奖活动 id
    user_id      integer      NOT NULL,               -- 获奖用户 id
    event_id     varchar(64)  NOT NULL,               -- 客户端分享事件标识（幂等键）
    channel      varchar(32)  NOT NULL DEFAULT '',    -- 分享渠道：weapp / h5_copy / web_share 等原文存
    reward_count integer      NOT NULL DEFAULT 0,     -- 本次入账的奖励抽奖次数（发放时配置快照）
    share_date   varchar(10)  NOT NULL DEFAULT '',    -- 分享日期 yyyy-MM-dd（UTC+8，与 lottery_user_stat.today_date 同口径）
    create_time  integer      NOT NULL DEFAULT 0      -- 入账时间（秒级时间戳）
);
CREATE UNIQUE INDEX IF NOT EXISTS uk_lottery_share_reward_event
    ON lottery_share_reward_log (activity_id, user_id, event_id);
CREATE INDEX IF NOT EXISTS idx_lottery_share_reward_user_day
    ON lottery_share_reward_log (activity_id, user_id, share_date);
CREATE INDEX IF NOT EXISTS idx_lottery_share_reward_activity_time
    ON lottery_share_reward_log (activity_id, create_time DESC);

COMMIT;
