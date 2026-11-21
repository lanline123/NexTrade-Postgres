-- ============================================================
-- 017_salesman_material_media_type.sql
-- 分销素材支持视频（我方新增：论坛 B 类缓做项第 38 条，帖 867768；
-- 原版 salesman_material 只有图文形态、无媒体类型概念）。
-- 素材表加三列：media_type（1 图文 / 2 视频）、video_url（视频地址）、
-- video_cover（视频封面图地址）。存量行 media_type 默认 1（图文），
-- 行为与此前逐字一致，video 两列为空。
-- 视频文件本体走现有素材中心视频库上传链（gallery_video_info 口径：
-- 本地上传仅 mp4、外链 mp4/flv/webm/avi、大小上限走上传配置），
-- 本表只存素材中心回给的 URL 与封面 URL，不新造上传通道。
-- media_type 用 INT（与实体 i32 解码对齐，避免 SMALLINT 解码错位）。
-- 注意：迁移执行器按分号逐条执行且会剥离 -- 注释行，语句内勿写分号。
-- ============================================================

ALTER TABLE salesman_material ADD COLUMN IF NOT EXISTS media_type INT NOT NULL DEFAULT 1;

ALTER TABLE salesman_material ADD COLUMN IF NOT EXISTS video_url VARCHAR(1000);

ALTER TABLE salesman_material ADD COLUMN IF NOT EXISTS video_cover VARCHAR(1000);
