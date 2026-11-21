-- ============================================================
-- 020_marketing_distribution.sql
-- 营销活动平台统一设置、门店同步（我方新增，论坛帖 177866）
-- 平台级营销活动（优惠券 / 满减）一键下发到目标店铺并生成店铺级
-- 活动副本：批次表 marketing_distribution + 逐店铺记录表
-- marketing_distribution_item。副本本体仍落 coupon /
-- product_promotion 原表（shop_id = 目标店铺），本两表只记下发
-- 关系、副本 id 与撤销状态，供下发记录查询与撤销定位。
-- 时间戳为秒级 INT（与既有表一致）。生产 auto_migrate = false，
-- 部署时先执行本 SQL 再发后端。
-- 注意：迁移执行器按分号逐条执行且会剥离注释行，语句内勿写分号。
-- ============================================================

CREATE TABLE IF NOT EXISTS marketing_distribution (
    distribution_id BIGSERIAL PRIMARY KEY,
    source_kind INT NOT NULL DEFAULT 0,
    source_id INT NOT NULL DEFAULT 0,
    source_name VARCHAR(255) NOT NULL DEFAULT '',
    target_count INT NOT NULL DEFAULT 0,
    success_count INT NOT NULL DEFAULT 0,
    skipped_count INT NOT NULL DEFAULT 0,
    failed_count INT NOT NULL DEFAULT 0,
    revoked_count INT NOT NULL DEFAULT 0,
    status INT NOT NULL DEFAULT 1,
    remark VARCHAR(500) NOT NULL DEFAULT '',
    admin_id INT NOT NULL DEFAULT 0,
    add_time INT NOT NULL DEFAULT 0,
    update_time INT NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS marketing_distribution_item (
    item_id BIGSERIAL PRIMARY KEY,
    distribution_id BIGINT NOT NULL DEFAULT 0,
    source_kind INT NOT NULL DEFAULT 0,
    source_id INT NOT NULL DEFAULT 0,
    target_shop_id INT NOT NULL DEFAULT 0,
    target_shop_name VARCHAR(255) NOT NULL DEFAULT '',
    copy_id INT NOT NULL DEFAULT 0,
    status INT NOT NULL DEFAULT 0,
    message VARCHAR(500) NOT NULL DEFAULT '',
    add_time INT NOT NULL DEFAULT 0,
    update_time INT NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_marketing_distribution_source
    ON marketing_distribution (source_kind, source_id, distribution_id DESC);

CREATE INDEX IF NOT EXISTS idx_marketing_distribution_item_batch
    ON marketing_distribution_item (distribution_id, item_id);

CREATE INDEX IF NOT EXISTS idx_marketing_distribution_item_dedupe
    ON marketing_distribution_item (source_kind, source_id, target_shop_id, status);
