-- 骑手管理完整版（论坛帖 487950 缓做项开工，我方新增）：骑手档案字段扩展。
-- 背景：service_staff 为原版表（字段止于姓名/电话/状态/完成量），入驻申请表
-- service_staff_apply 已留存 id_card 但审核生成人员时未带入；完整骑手管理需要
-- 档案字段（证件/性别/车辆/紧急联系人/入职时间/管理员备注）落到人员本体。
-- 出处：我方新增（原版 service_staff 无下列任一字段，实体由 java pure.sql 生成可证）。
-- 幂等：全部 IF NOT EXISTS，可重复执行。生产 auto_migrate=false，部署时先执行
-- 本 SQL 再发后端（实体与 VO 已含新列，旧二进制不读新列、不受影响）。
-- 编号说明：仓库现有最大编号为 015（015_site_clone_tables），本迁移顺延 016。
-- 若主聊合并时重排编号，改文件名即可（迁移执行器只认文件名，代码无引用）。
BEGIN;

ALTER TABLE service_staff
    ADD COLUMN IF NOT EXISTS id_card           varchar(30)  NOT NULL DEFAULT '',  -- 证件号（入驻审核通过时从申请带入，MVP 仅留存不做实名核验）
    ADD COLUMN IF NOT EXISTS gender            integer      NOT NULL DEFAULT 0,   -- 0 未知 1 男 2 女
    ADD COLUMN IF NOT EXISTS vehicle_type      varchar(30)  NOT NULL DEFAULT '',  -- 交通工具（电动车/摩托车/汽车等，自由文本）
    ADD COLUMN IF NOT EXISTS plate_no          varchar(20)  NOT NULL DEFAULT '',  -- 车牌号
    ADD COLUMN IF NOT EXISTS emergency_contact varchar(50)  NOT NULL DEFAULT '',  -- 紧急联系人
    ADD COLUMN IF NOT EXISTS emergency_phone   varchar(20)  NOT NULL DEFAULT '',  -- 紧急联系电话
    ADD COLUMN IF NOT EXISTS entry_time        bigint       NOT NULL DEFAULT 0,   -- 入职时间（epoch 秒；审核通过/后台建人时落）
    ADD COLUMN IF NOT EXISTS admin_remark      varchar(255) NOT NULL DEFAULT '';  -- 管理员备注（内部可见）

-- 轨迹点按时间窗查询（后台轨迹页按日期筛）补索引；既有 (staff_id, create_time)
-- 与 (delivery_order_id) 索引在 2026-11-20_rider_mvp.sql 已建，不重复。
CREATE INDEX IF NOT EXISTS idx_track_time ON staff_track_point (create_time);

COMMIT;
