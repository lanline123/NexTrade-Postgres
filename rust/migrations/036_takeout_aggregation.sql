-- 外卖聚合一期（A 向订单汇入框架 + 美团闪购接入，我方新增）。
-- 设计：~/workspace/takeout-aggregation/DESIGN.md（用户 2026-11-21 05:57 拍板按分期办；
-- 一期 = 聚合框架本体 + 美团闪购直连，凭据未到位时全链诚实未启用、不假成功；
-- B 向麦芽田只留接口位，本迁移不含 B 向表）。
-- 幂等：CREATE TABLE IF NOT EXISTS / ADD COLUMN IF NOT EXISTS，可重复执行。
-- 生产 auto_migrate=false，部署时先执行本 SQL 再发后端（新二进制实体含
-- order.external_channel / external_order_sn 两列，旧二进制不读新列，前后兼容）。
-- 编号说明：027 热更新线、028 代客下单接单大厅（028_proxy_hall.sql）、
-- 029 下单提交幂等键（029_order_idempotency.sql）均已在 main 占用，
-- 本文件取 030；若站点搬家 P2 的 site_clone_asset_map 迁移先行落地
-- 占用 030，本文件顺延 031——改文件名即可（迁移执行器只认文件名，
-- 代码无引用，头注与 CHANGELOG 措辞同步改）。

BEGIN;

-- 1) 渠道门店绑定：ISV 应用 + 商家逐店授权模型（设计 §1.4.1）。
--    app_poi_code = 我方侧门店编码，是美团推送与 API 调用的绑定键；
--    access_token 为商家 OAuth 授权令牌（ISV 应用签名须带），未授权为空。
CREATE TABLE IF NOT EXISTS takeout_channel_binding (
    id               serial PRIMARY KEY,
    channel          varchar(32)  NOT NULL DEFAULT '',   -- meituan / eleme / douyin
    shop_id          integer      NOT NULL DEFAULT 0,    -- 我方门店（shop.id）
    app_poi_code     varchar(64)  NOT NULL DEFAULT '',   -- 我方侧门店编码（渠道绑定键）
    external_poi_id  varchar(64)  NOT NULL DEFAULT '',   -- 渠道侧门店 id（授权回填）
    external_poi_name varchar(255) NOT NULL DEFAULT '',  -- 渠道侧门店名（展示）
    auth_status      integer      NOT NULL DEFAULT 0,    -- 0 未授权 / 1 已授权 / 2 已失效
    access_token     text         NOT NULL DEFAULT '',
    auto_accept      integer      NOT NULL DEFAULT 1,    -- 新单自动确认（美团 5 分钟接单红线，设计 §1.2）
    enabled          integer      NOT NULL DEFAULT 1,
    add_time         integer      NOT NULL DEFAULT 0,
    update_time      integer      NOT NULL DEFAULT 0
);
CREATE UNIQUE INDEX IF NOT EXISTS uk_takeout_binding_channel_poi
    ON takeout_channel_binding (channel, app_poi_code);
CREATE INDEX IF NOT EXISTS idx_takeout_binding_shop
    ON takeout_channel_binding (shop_id, channel);

-- 2) 渠道 SKU 映射：渠道 SKU 与我方 SKU 无天然对应，映射是长期运维项（设计 §1.4.4）。
--    未映射的订单行按渠道原始商品名落单（product_id = 0），事后人工补映射。
CREATE TABLE IF NOT EXISTS takeout_sku_mapping (
    id              serial PRIMARY KEY,
    channel         varchar(32)  NOT NULL DEFAULT '',
    shop_id         integer      NOT NULL DEFAULT 0,
    external_spu_id varchar(64)  NOT NULL DEFAULT '',   -- 渠道商品 id（app_spu_code 等）
    external_sku_id varchar(64)  NOT NULL DEFAULT '',   -- 渠道 SKU id（app_food_code / sku_id）
    external_name   varchar(500) NOT NULL DEFAULT '',   -- 渠道商品名快照（辅助人工匹配）
    product_id      integer      NOT NULL DEFAULT 0,    -- 我方商品
    sku_id          integer      NOT NULL DEFAULT 0,    -- 我方 SKU（0 = 无规格/默认）
    add_time        integer      NOT NULL DEFAULT 0,
    update_time     integer      NOT NULL DEFAULT 0
);
CREATE UNIQUE INDEX IF NOT EXISTS uk_takeout_mapping_channel_shop_sku
    ON takeout_sku_mapping (channel, shop_id, external_sku_id);
CREATE INDEX IF NOT EXISTS idx_takeout_mapping_product
    ON takeout_sku_mapping (product_id, sku_id);

-- 3) 外部订单主记录：渠道推送的归一落库（金额一律以渠道推送为准，不按我方口径重算，
--    设计 §1.2 / §1.4.2）。internal_order_id > 0 表示已在我方订单表生成履约单
--    （order_type = 13，external_channel / external_order_sn 双标记，资金链隔离）。
--    status 为归一状态：paid / confirmed / completed / cancelled / refunding / refunded /
--    pending_binding（门店未绑定，待人工处理后 reprocess）。
CREATE TABLE IF NOT EXISTS takeout_external_order (
    id                serial PRIMARY KEY,
    channel           varchar(32)   NOT NULL DEFAULT '',
    external_order_id varchar(64)   NOT NULL DEFAULT '',
    shop_id           integer       NOT NULL DEFAULT 0,
    binding_id        integer       NOT NULL DEFAULT 0,
    internal_order_id integer       NOT NULL DEFAULT 0,
    external_status   integer       NOT NULL DEFAULT 0,   -- 渠道原始状态码（美团 1/2/4/8/9）
    status            varchar(32)   NOT NULL DEFAULT '',
    total_amount      numeric(12,2) NOT NULL DEFAULT 0,   -- 渠道口径总额（元，已由分换算）
    product_amount    numeric(12,2) NOT NULL DEFAULT 0,
    shipping_fee      numeric(12,2) NOT NULL DEFAULT 0,
    consignee         varchar(255)  NOT NULL DEFAULT '',
    mobile            varchar(64)   NOT NULL DEFAULT '',  -- 渠道隐私号原样落库
    address           text          NOT NULL DEFAULT '',
    raw_payload       text          NOT NULL DEFAULT '',  -- 最新一次推送的归一 JSON（原文见回调日志）
    last_event        varchar(64)   NOT NULL DEFAULT '',
    last_error        varchar(500)  NOT NULL DEFAULT '',
    add_time          integer       NOT NULL DEFAULT 0,
    update_time       integer       NOT NULL DEFAULT 0
);
CREATE UNIQUE INDEX IF NOT EXISTS uk_takeout_ext_order
    ON takeout_external_order (channel, external_order_id);
CREATE INDEX IF NOT EXISTS idx_takeout_ext_order_shop
    ON takeout_external_order (shop_id, id DESC);

-- 4) 回调日志：每次推送先落日志后处理（验签结果 / 处理结果 / 原文），
--    兼作审计与幂等排障（渠道失败重推 3 次，设计与官方文档口径）。
CREATE TABLE IF NOT EXISTS takeout_callback_log (
    id                serial PRIMARY KEY,
    channel           varchar(32)  NOT NULL DEFAULT '',
    event_type        varchar(64)  NOT NULL DEFAULT '',   -- order / cancel / refund / status / unknown
    external_order_id varchar(64)  NOT NULL DEFAULT '',
    sign_verified     integer      NOT NULL DEFAULT 0,    -- 0 未验/失败 / 1 通过
    process_status    integer      NOT NULL DEFAULT 0,    -- 0 成功 / 1 处理异常 / 2 拒绝（未配置/验签失败）
    message           varchar(500) NOT NULL DEFAULT '',
    raw_body          text         NOT NULL DEFAULT '',
    add_time          integer      NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS idx_takeout_callback_order
    ON takeout_callback_log (channel, external_order_id, id DESC);
CREATE INDEX IF NOT EXISTS idx_takeout_callback_time
    ON takeout_callback_log (id DESC);

-- 5) 订单表外部标记：order_type = 13 已表达「外部渠道单」，再加两列做渠道溯源与
--    资金链隔离判据（结算/分销/售后出款侧以 external_channel <> '' 排除，设计 §1.4.2）。
ALTER TABLE "order" ADD COLUMN IF NOT EXISTS external_channel  varchar(32) NOT NULL DEFAULT '';
ALTER TABLE "order" ADD COLUMN IF NOT EXISTS external_order_sn varchar(64) NOT NULL DEFAULT '';
CREATE INDEX IF NOT EXISTS idx_order_external_channel
    ON "order" (external_channel, external_order_sn) WHERE external_channel <> '';

COMMIT;
