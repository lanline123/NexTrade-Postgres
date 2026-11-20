-- 2026-11-18: 商品对接 API（Product Feed）供货侧表。
-- 设计文档：~/workspace/product-feed-api-design/proposal.md（P1）。
-- 时间列统一 integer Unix 秒（与 product.add_time / last_update 同口径）；
-- 金额 numeric(12,2)；secret 只存 SHA-256 摘要，明文仅创建/重置时返回一次。
BEGIN;

-- 对接密钥主表：一把密钥 = 一个下游（身份 + 授权范围 + 默认价格规则）
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

-- 指定商品范围（scope_mode=2 时生效）
CREATE TABLE IF NOT EXISTS product_feed_key_product (
    id          serial PRIMARY KEY,
    feed_key_id integer NOT NULL,
    product_id  integer NOT NULL
);
CREATE UNIQUE INDEX IF NOT EXISTS uk_feed_key_product ON product_feed_key_product (feed_key_id, product_id);
CREATE INDEX IF NOT EXISTS idx_feed_key_product_key ON product_feed_key_product (feed_key_id);

-- 指定分类范围（scope_mode=3 时生效；存原始勾选分类，查询时展开全部后代）
CREATE TABLE IF NOT EXISTS product_feed_key_category (
    id          serial PRIMARY KEY,
    feed_key_id integer NOT NULL,
    category_id integer NOT NULL
);
CREATE UNIQUE INDEX IF NOT EXISTS uk_feed_key_category ON product_feed_key_category (feed_key_id, category_id);
CREATE INDEX IF NOT EXISTS idx_feed_key_category_key ON product_feed_key_category (feed_key_id);

-- 单品价格规则覆盖（优先级高于密钥默认规则）
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

COMMIT;
