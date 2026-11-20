-- 2026-11-20: 自营骑手 MVP（我方新增激活层；地基表 service_staff / delivery_order /
-- service_staff_wallet 等为原版 5.8.57+ 已移植表，本迁移只做增量列与新表）。
-- 类型口径：禁用 SMALLINT；枚举/标志一律 INT；金额 NUMERIC(12,2)；时间戳 BIGINT（epoch 秒）。
-- 阶段一：门店默认服务费两级 + 派送单服务费快照列。
-- 阶段二：入驻申请表 service_staff_apply。
-- 阶段三：状态动作点定位 staff_track_point + 最新位置 staff_location_latest。
BEGIN;

-- ——— 阶段一 ———
-- 门店默认服务费（费率三级链的第二级：SKU/商品级 → 门店默认 → 平台统一费率）。
-- type: 0=固定金额（元/件） 1=按比例（百分比，沿用本仓佣金 rate/100 口径）；value=0 视为未设置。
ALTER TABLE shop_local_delivery_config
    ADD COLUMN IF NOT EXISTS default_service_fee_type  integer       NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS default_service_fee_value numeric(12,2) NOT NULL DEFAULT 0;

ALTER TABLE shop_home_service_config
    ADD COLUMN IF NOT EXISTS default_service_fee_type  integer       NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS default_service_fee_value numeric(12,2) NOT NULL DEFAULT 0;

-- 派送完成入账的服务费快照（门店成本留痕 + 对账 + 幂等复核）。
ALTER TABLE delivery_order
    ADD COLUMN IF NOT EXISTS service_fee numeric(12,2) NOT NULL DEFAULT 0;

-- ——— 阶段二：入驻申请 ———
-- 会员申请成为某店铺的服务人员（type 1=配送员/2=服务员），审核通过生成 service_staff。
CREATE TABLE IF NOT EXISTS service_staff_apply (
    apply_id    serial PRIMARY KEY,
    member_id   integer NOT NULL DEFAULT 0,          -- 申请会员（user.user_id）
    shop_id     integer NOT NULL DEFAULT 0,          -- 申请入驻店铺（shop.shop_id）
    type        integer NOT NULL DEFAULT 1,          -- 1=配送员 2=服务员
    name        varchar(50)  NOT NULL DEFAULT '',
    phone       varchar(20)  NOT NULL DEFAULT '',
    id_card     varchar(30)  NOT NULL DEFAULT '',    -- 证件号（MVP 仅留存，不做实名核验）
    remark      varchar(255) NOT NULL DEFAULT '',    -- 申请备注
    status      integer NOT NULL DEFAULT 0,          -- 0=待审核 1=已通过 2=已驳回
    audit_remark varchar(255) NOT NULL DEFAULT '',   -- 审核备注/驳回原因
    audit_admin_id integer NOT NULL DEFAULT 0,       -- 审核人 admin_user id
    staff_id    integer NOT NULL DEFAULT 0,          -- 通过后生成的 service_staff.id
    create_time bigint NOT NULL DEFAULT 0,
    update_time bigint NOT NULL DEFAULT 0,
    is_delete   integer NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS idx_staff_apply_shop ON service_staff_apply (shop_id, status, is_delete);
CREATE INDEX IF NOT EXISTS idx_staff_apply_member ON service_staff_apply (member_id, is_delete);

-- ——— 阶段三：定位（仅状态动作点打点，无连续轨迹） ———
CREATE TABLE IF NOT EXISTS staff_track_point (
    point_id         serial PRIMARY KEY,
    staff_id         integer NOT NULL DEFAULT 0,
    delivery_order_id integer NOT NULL DEFAULT 0,
    action           varchar(20) NOT NULL DEFAULT '',  -- grab/pickup/complete
    longitude        numeric(12,7),
    latitude         numeric(12,7),
    create_time      bigint NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS idx_track_staff ON staff_track_point (staff_id, create_time);
CREATE INDEX IF NOT EXISTS idx_track_order ON staff_track_point (delivery_order_id);

CREATE TABLE IF NOT EXISTS staff_location_latest (
    staff_id    integer PRIMARY KEY,
    longitude   numeric(12,7),
    latitude    numeric(12,7),
    update_time bigint NOT NULL DEFAULT 0
);

COMMIT;
