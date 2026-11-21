-- 微商「店中店」分销员外壳店（论坛帖 257876041323848173，我方新增）—— 2026-11-21 立项批次。
--
-- ⚠️ 本文件由店中店实施线备好，**未执行**。部署含店中店代码的二进制前
-- 必须先跑本迁移：新代码会读写 salesman_shop 表，未迁移的库上
-- /api/salesman/shop/* 与 /adminapi/salesman/shop/* 会因表不存在而失败
-- （部署顺序约束）。
--
-- 内容：
--   salesman_shop：分销员外壳店的**呈现配置**（一分销员一行，salesman_id 主键）。
--   只是呈现 + 归因层：不建独立店铺实体、不复制商品/库存/订单体系；
--   商品/下单/售后仍走主商城既有链，订单归因与佣金走既有分销绑定
--   （salesman_customer）与佣金生成链，本表不参与任何资金口径。
--   - shop_name / banner / intro：分销员可自定义的店名 / 店招图 / 简介（MVP 三项）。
--   - is_enabled：平台禁用开关（1 展示 / 0 禁用）。禁用只关外壳展示，
--     不动分销员身份、客户绑定与佣金（与分销员禁用是两套概念）。
--   无行 = 未自定义：买家侧按分销员昵称回退展示、默认启用。
--
-- 类型口径（本仓现行）：无 SMALLINT；id/标志 INT；时间 INT epoch 秒。

BEGIN;

CREATE TABLE IF NOT EXISTS salesman_shop (
    salesman_id integer PRIMARY KEY,
    shop_name   varchar(100) NOT NULL DEFAULT '',
    banner      varchar(1000) NOT NULL DEFAULT '',
    intro       varchar(500)  NOT NULL DEFAULT '',
    is_enabled  integer       NOT NULL DEFAULT 1,
    add_time    integer       NOT NULL DEFAULT 0,
    update_time integer       NOT NULL DEFAULT 0
);

COMMENT ON TABLE salesman_shop IS '分销员外壳店呈现配置（店中店，我方新增）：店名/店招/简介 + 平台展示开关；仅呈现+归因，不参与资金口径';
COMMENT ON COLUMN salesman_shop.is_enabled IS '平台展示开关：1 展示 0 禁用；禁用只关外壳展示，不动分销身份/绑定/佣金';

COMMIT;
