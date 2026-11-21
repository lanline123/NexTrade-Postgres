-- 平台联邦（hub）三期③：跨店下单（设计 §7.2）。
-- 设计：~/workspace/federation-phase3/DESIGN.md §3；上位设计
-- ~/workspace/hub-federation-design/proposal.md §7.2。
-- 资金口径：消费者付款只走中枢本站既有支付通道（paylog order_type=3）；
-- 分账只记账目（结算批 046），不执行渠道拆分（收付通已搁置）。
-- 幂等：全部 IF NOT EXISTS，可重复执行。生产 auto_migrate=false，
-- 部署时先执行本 SQL 再发后端。
BEGIN;

-- 中枢侧跨店主单：一单可含多个成员站的多家店铺（按连接+远端店铺拆子单）。
CREATE TABLE IF NOT EXISTS hub_xorder (
    id             serial PRIMARY KEY,
    xorder_sn      varchar(40)   NOT NULL DEFAULT '',  -- HX 前缀单号
    buyer_user_id  integer       NOT NULL DEFAULT 0,   -- 中枢本地买家
    global_member_id varchar(40) NOT NULL DEFAULT '',  -- 买家全局 ID（下单时解析快照，可空）
    product_amount numeric(12,2) NOT NULL DEFAULT 0,
    shipping_fee   numeric(12,2) NOT NULL DEFAULT 0,
    total_amount   numeric(12,2) NOT NULL DEFAULT 0,
    pay_status     smallint      NOT NULL DEFAULT 0,   -- 0 未付 / 1 已付 / 2 已退款
    status         smallint      NOT NULL DEFAULT 0,   -- 0 待付款 / 1 已付待路由 / 2 已全部路由 / 3 已完成 / 4 已关闭 / 5 售后中
    paylog_id      integer       NOT NULL DEFAULT 0,
    pay_sn         varchar(40)   NOT NULL DEFAULT '',
    consignee      varchar(100)  NOT NULL DEFAULT '',  -- 收货快照（下单时从买家地址簿固化）
    mobile         varchar(32)   NOT NULL DEFAULT '',
    address        varchar(500)  NOT NULL DEFAULT '',
    add_time       integer       NOT NULL DEFAULT 0,
    pay_time       integer       NOT NULL DEFAULT 0,
    update_time    integer       NOT NULL DEFAULT 0,
    UNIQUE (xorder_sn)
);
CREATE INDEX IF NOT EXISTS idx_hub_xorder_buyer ON hub_xorder (buyer_user_id, add_time);

-- 中枢侧跨店子单：按 (连接, 远端店铺) 拆分，一子单对成员侧一张本地单。
CREATE TABLE IF NOT EXISTS hub_xorder_sub (
    id              serial PRIMARY KEY,
    master_id       integer       NOT NULL DEFAULT 0,
    sub_sn          varchar(48)   NOT NULL DEFAULT '',  -- 主单号-序号
    link_id         integer       NOT NULL DEFAULT 0,
    remote_shop_id  integer       NOT NULL DEFAULT 0,
    remote_shop_name varchar(200) NOT NULL DEFAULT '',
    amount          numeric(12,2) NOT NULL DEFAULT 0,   -- 商品额（店主确认价合计）
    shipping_fee    numeric(12,2) NOT NULL DEFAULT 0,
    status          smallint      NOT NULL DEFAULT 0,
    -- 0 待预占 / 1 已预占 / 2 已路由 / 3 已发货 / 4 已完成 / 5 店主拒单 /
    -- 6 预占失败 / 7 已退款 / 8 退款申请中 / 9 已关闭（买家取消/超时释放，设计外补值）
    remote_order_id integer       NOT NULL DEFAULT 0,   -- 路由后成员侧本地单
    remote_order_sn varchar(40)   NOT NULL DEFAULT '',
    tracking_no     varchar(64)   NOT NULL DEFAULT '',
    logistics_name  varchar(100)  NOT NULL DEFAULT '',
    add_time        integer       NOT NULL DEFAULT 0,
    update_time     integer       NOT NULL DEFAULT 0,
    UNIQUE (sub_sn)
);
CREATE INDEX IF NOT EXISTS idx_hub_xorder_sub_master ON hub_xorder_sub (master_id);
CREATE INDEX IF NOT EXISTS idx_hub_xorder_sub_link ON hub_xorder_sub (link_id, status);

-- 中枢侧跨店单行：价格为成员预占时回的确认价快照。
CREATE TABLE IF NOT EXISTS hub_xorder_item (
    id               serial PRIMARY KEY,
    sub_id           integer       NOT NULL DEFAULT 0,
    map_id           integer       NOT NULL DEFAULT 0,  -- 聚合映射 hub_product_map.map_id
    remote_product_id integer      NOT NULL DEFAULT 0,
    remote_sku_id    integer       NOT NULL DEFAULT 0,  -- 聚合为商品级口径，本批恒 0
    product_name     varchar(255)  NOT NULL DEFAULT '',
    pic_url          varchar(500)  NOT NULL DEFAULT '',
    price            numeric(12,2) NOT NULL DEFAULT 0,  -- 店主确认价
    quantity         integer       NOT NULL DEFAULT 0,
    amount           numeric(12,2) NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS idx_hub_xorder_item_sub ON hub_xorder_item (sub_id);

-- 成员侧库存预占：子单号维度，TTL 15 分钟；路由时消费、取消/超时释放。
CREATE TABLE IF NOT EXISTS hub_stock_hold (
    id              serial PRIMARY KEY,
    link_id         integer     NOT NULL DEFAULT 0,  -- 来源中枢连接（成员视角 role=2）
    hold_sn         varchar(48) NOT NULL DEFAULT '', -- = 中枢子单号
    product_id      integer     NOT NULL DEFAULT 0,
    sku_id          integer     NOT NULL DEFAULT 0,
    quantity        integer     NOT NULL DEFAULT 0,
    status          smallint    NOT NULL DEFAULT 0,  -- 0 有效 / 1 已消费 / 2 已释放 / 3 已过期
    expire_time     integer     NOT NULL DEFAULT 0,
    remote_order_id integer     NOT NULL DEFAULT 0,  -- 消费后落的本地订单 id
    add_time        integer     NOT NULL DEFAULT 0,
    update_time     integer     NOT NULL DEFAULT 0,
    UNIQUE (link_id, hold_sn, product_id, sku_id)
);
CREATE INDEX IF NOT EXISTS idx_hub_stock_hold_product ON hub_stock_hold (product_id, status);

-- 成员侧路由落单登记：子单号幂等键，镜像子单状态供后台同步。
CREATE TABLE IF NOT EXISTS hub_xorder_route (
    id                   serial PRIMARY KEY,
    link_id              integer      NOT NULL DEFAULT 0,
    sub_sn               varchar(48)  NOT NULL DEFAULT '',
    local_order_id       integer      NOT NULL DEFAULT 0,
    local_order_sn       varchar(40)  NOT NULL DEFAULT '',
    buyer_global_member_id varchar(40) NOT NULL DEFAULT '',
    status               smallint     NOT NULL DEFAULT 2, -- 镜像中枢子单状态（落单即 2 已路由）
    add_time             integer      NOT NULL DEFAULT 0,
    update_time          integer      NOT NULL DEFAULT 0,
    UNIQUE (sub_sn)
);
CREATE INDEX IF NOT EXISTS idx_hub_xorder_route_link ON hub_xorder_route (link_id, add_time);

COMMIT;
