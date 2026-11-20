-- 003_product_feed_tables.sql
-- 商品对接 API（Product Feed）供货侧表，与 rust/deploy/pg/2026-11-18_product_feed.sql 同一份 DDL。
-- 注意：run_migrations 按分号切分逐条执行，本文件语句内不得在字符串字面量中出现分号。

CREATE TABLE IF NOT EXISTS product_feed_key (
    feed_key_id     serial PRIMARY KEY,
    key_name        varchar(100) NOT NULL,
    access_key      varchar(32)  NOT NULL,
    secret_hash     varchar(64)  NOT NULL,
    status          smallint     NOT NULL DEFAULT 1,
    scope_mode      smallint     NOT NULL DEFAULT 1,
    include_inquiry smallint     NOT NULL DEFAULT 0,
    price_rule_type varchar(16)  NOT NULL DEFAULT 'none',
    price_rule_value numeric(12,2) NOT NULL DEFAULT 0,
    expire_time     integer      NOT NULL DEFAULT 0,
    remark          varchar(255) NOT NULL DEFAULT '',
    shop_id         integer      NOT NULL DEFAULT 0,
    last_used_time  integer      NOT NULL DEFAULT 0,
    call_count      bigint       NOT NULL DEFAULT 0,
    add_time        integer      NOT NULL DEFAULT 0,
    update_time     integer      NOT NULL DEFAULT 0
);

CREATE UNIQUE INDEX IF NOT EXISTS uk_product_feed_key_access_key ON product_feed_key (access_key);

CREATE INDEX IF NOT EXISTS idx_product_feed_key_status ON product_feed_key (status);

CREATE TABLE IF NOT EXISTS product_feed_key_product (
    id          serial PRIMARY KEY,
    feed_key_id integer NOT NULL,
    product_id  integer NOT NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS uk_feed_key_product ON product_feed_key_product (feed_key_id, product_id);

CREATE INDEX IF NOT EXISTS idx_feed_key_product_key ON product_feed_key_product (feed_key_id);

CREATE TABLE IF NOT EXISTS product_feed_key_category (
    id          serial PRIMARY KEY,
    feed_key_id integer NOT NULL,
    category_id integer NOT NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS uk_feed_key_category ON product_feed_key_category (feed_key_id, category_id);

CREATE INDEX IF NOT EXISTS idx_feed_key_category_key ON product_feed_key_category (feed_key_id);

CREATE TABLE IF NOT EXISTS product_feed_price_override (
    id          serial PRIMARY KEY,
    feed_key_id integer NOT NULL,
    product_id  integer NOT NULL,
    rule_type   varchar(16)   NOT NULL DEFAULT 'none',
    rule_value  numeric(12,2) NOT NULL DEFAULT 0,
    add_time    integer NOT NULL DEFAULT 0,
    update_time integer NOT NULL DEFAULT 0
);

CREATE UNIQUE INDEX IF NOT EXISTS uk_feed_override ON product_feed_price_override (feed_key_id, product_id);

CREATE INDEX IF NOT EXISTS idx_feed_override_key ON product_feed_price_override (feed_key_id);
