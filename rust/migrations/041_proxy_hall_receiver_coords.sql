-- ============================================================
-- 041_proxy_hall_receiver_coords.sql
-- 接单大厅订单池距离补齐（我方新增，二期登记缺口收尾）：
-- 大厅池行此前没有收货地址坐标，池列表 distance_meters 恒空、
-- 无法按距离排序。本迁移给 proxy_order_ext 加收货坐标快照列，
-- 大厅单提交时把收货地址的经纬度快照落行（best-effort：地址无
-- 坐标/自提单留空），池列表按查看门店坐标实时算球面距离展示
-- 并可按距离升序排序。存量单无快照、距离为空属正常口径，
-- 不回填造数；一期直选单不写快照、行为逐字不变。
-- 部署顺序：本迁移必须先于新二进制换版执行（新版会读写新列，
-- 旧版对新列不敏感，反序安全、正序必需）。
-- 注意：迁移执行器按分号逐条执行且会剥离 -- 注释行，语句内勿写分号。
-- ============================================================

ALTER TABLE proxy_order_ext ADD COLUMN IF NOT EXISTS receiver_longitude DECIMAL(10,6);
ALTER TABLE proxy_order_ext ADD COLUMN IF NOT EXISTS receiver_latitude DECIMAL(10,6);
