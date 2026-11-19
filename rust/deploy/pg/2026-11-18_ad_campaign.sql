-- 2026-11-18: 店铺街千人千面 + 商家自助投流（店铺/商品）。
-- 投放单表 ad_campaign：平台置顶与付费推广同表不同来源（is_pin=1 为平台置顶，amount=0）。
-- 定价/开关存 config.biz_code = 'adSettings'（JSON），无需建表。
BEGIN;

CREATE TABLE IF NOT EXISTS ad_campaign (
    id             serial PRIMARY KEY,
    target_type    integer NOT NULL DEFAULT 1,
    shop_id        integer NOT NULL DEFAULT 0,
    product_id     integer NOT NULL DEFAULT 0,
    position       varchar(32) NOT NULL DEFAULT 'shop_street',
    is_pin         integer NOT NULL DEFAULT 0,
    pin_sort       integer NOT NULL DEFAULT 0,
    start_time     integer NOT NULL DEFAULT 0,
    end_time       integer NOT NULL DEFAULT 0,
    days           integer NOT NULL DEFAULT 0,
    unit_price     decimal(12, 2) NOT NULL DEFAULT 0,
    amount         decimal(12, 2) NOT NULL DEFAULT 0,
    weight         integer NOT NULL DEFAULT 0,
    pay_status     integer NOT NULL DEFAULT 0,
    pay_channel    varchar(32) NOT NULL DEFAULT '',
    pay_time       integer NOT NULL DEFAULT 0,
    account_log_id integer NOT NULL DEFAULT 0,
    audit_status   integer NOT NULL DEFAULT 0,
    audit_remark   varchar(255) NOT NULL DEFAULT '',
    audit_admin_id integer NOT NULL DEFAULT 0,
    audit_time     integer NOT NULL DEFAULT 0,
    status         integer NOT NULL DEFAULT 0,
    add_time       integer NOT NULL DEFAULT 0,
    update_time    integer NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS idx_ad_campaign_shop ON ad_campaign (shop_id, id DESC);
CREATE INDEX IF NOT EXISTS idx_ad_campaign_active ON ad_campaign (position, audit_status, pay_status, status, start_time, end_time);
CREATE INDEX IF NOT EXISTS idx_ad_campaign_audit ON ad_campaign (audit_status, id DESC);

COMMIT;
