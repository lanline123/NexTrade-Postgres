-- 012 礼品卡适用范围（论坛 B 类 227890，我方新增）：储值卡卡组可限定仅适用于
-- 指定商品或指定品类，结算抵扣基数只计范围内商品金额（含其按比例分摊的
-- 运费/税费，与拆单 allocated_amount 同口径）。兑换卡由绑定商品池天然限定，
-- 不使用本范围字段。
-- 范围挂卡组（动态生效，与面额/兑换字段同为卡组级口径）；scope_type：
-- 0=全部商品（默认，与现行行为一致，存量零变化）/ 1=指定商品 / 2=指定品类
-- （含子类目，按 category 父子树展开）。

ALTER TABLE gift_card_groups ADD COLUMN IF NOT EXISTS scope_type integer NOT NULL DEFAULT 0;

-- 指定商品范围（卡组 → 商品多对多）
CREATE TABLE IF NOT EXISTS gift_card_group_scope_products (
    id             bigserial PRIMARY KEY,
    card_groups_id integer NOT NULL,
    product_id     integer NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_gcg_scope_products_group ON gift_card_group_scope_products (card_groups_id);
CREATE UNIQUE INDEX IF NOT EXISTS uk_gcg_scope_products ON gift_card_group_scope_products (card_groups_id, product_id);

-- 指定品类范围（卡组 → 品类多对多，存运营勾选的原始品类，子类目在计算时展开）
CREATE TABLE IF NOT EXISTS gift_card_group_scope_categories (
    id             bigserial PRIMARY KEY,
    card_groups_id integer NOT NULL,
    category_id    integer NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_gcg_scope_categories_group ON gift_card_group_scope_categories (card_groups_id);
CREATE UNIQUE INDEX IF NOT EXISTS uk_gcg_scope_categories ON gift_card_group_scope_categories (card_groups_id, category_id);
