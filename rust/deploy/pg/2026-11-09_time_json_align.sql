-- ============================================================
-- 2026-11-09 时间/JSON 列类型对齐（varchar -> timestamp/date/jsonb）
--
-- ⚠️ 必须与新版二进制同时切换：当前线上二进制把这些列按 String 解码，
--    单独执行本脚本会让 cart / order / promotion / user 等功能立即报错。
--
-- 切换步骤（约 10-30 秒不可用）：
--   1. 部署新版 nextrade 二进制（先用 systemctl stop）
--   2. psql -f 本文件
--   3. systemctl start nextrade；curl /adminapi/health 与 /api/health
-- 回滚：把列改回 varchar/text（见文件末尾注释）。
--
-- 数据预检（2026-11-09）：空串 0、非法格式 0；仅 3 列有数据：
--   wechat_live.create_time / statistics_base.date / user.birthday
-- ============================================================

BEGIN;
SET LOCAL lock_timeout = '10s';

ALTER TABLE "category_service_fee_rule" ALTER COLUMN "create_time" TYPE timestamp USING NULLIF(btrim("create_time"), '')::timestamp;
ALTER TABLE "category_service_fee_rule" ALTER COLUMN "modify_time" TYPE timestamp USING NULLIF(btrim("modify_time"), '')::timestamp;
ALTER TABLE "category_service_fee_rule_category" ALTER COLUMN "create_time" TYPE timestamp USING NULLIF(btrim("create_time"), '')::timestamp;
ALTER TABLE "category_service_fee_rule_category" ALTER COLUMN "modify_time" TYPE timestamp USING NULLIF(btrim("modify_time"), '')::timestamp;
ALTER TABLE "category_service_fee_rule_target" ALTER COLUMN "create_time" TYPE timestamp USING NULLIF(btrim("create_time"), '')::timestamp;
ALTER TABLE "category_service_fee_rule_target" ALTER COLUMN "modify_time" TYPE timestamp USING NULLIF(btrim("modify_time"), '')::timestamp;
ALTER TABLE "merchant_application" ALTER COLUMN "audit_time" TYPE timestamp USING NULLIF(btrim("audit_time"), '')::timestamp;
ALTER TABLE "merchant_application" ALTER COLUMN "create_time" TYPE timestamp USING NULLIF(btrim("create_time"), '')::timestamp;
ALTER TABLE "merchant_application" ALTER COLUMN "modify_time" TYPE timestamp USING NULLIF(btrim("modify_time"), '')::timestamp;
ALTER TABLE "merchant_application_media" ALTER COLUMN "next_retry_time" TYPE timestamp USING NULLIF(btrim("next_retry_time"), '')::timestamp;
ALTER TABLE "merchant_application_media" ALTER COLUMN "expire_time" TYPE timestamp USING NULLIF(btrim("expire_time"), '')::timestamp;
ALTER TABLE "merchant_application_media" ALTER COLUMN "create_time" TYPE timestamp USING NULLIF(btrim("create_time"), '')::timestamp;
ALTER TABLE "merchant_application_media" ALTER COLUMN "modify_time" TYPE timestamp USING NULLIF(btrim("modify_time"), '')::timestamp;
ALTER TABLE "product_cycle" ALTER COLUMN "validity_start_date" TYPE timestamp USING NULLIF(btrim("validity_start_date"), '')::timestamp;
ALTER TABLE "product_cycle" ALTER COLUMN "validity_end_date" TYPE timestamp USING NULLIF(btrim("validity_end_date"), '')::timestamp;
ALTER TABLE "profit_sharing_order" ALTER COLUMN "finish_time" TYPE timestamp USING NULLIF(btrim("finish_time"), '')::timestamp;
ALTER TABLE "profit_sharing_order" ALTER COLUMN "create_time" TYPE timestamp USING NULLIF(btrim("create_time"), '')::timestamp;
ALTER TABLE "profit_sharing_order" ALTER COLUMN "modify_time" TYPE timestamp USING NULLIF(btrim("modify_time"), '')::timestamp;
ALTER TABLE "profit_sharing_return_order" ALTER COLUMN "create_time" TYPE timestamp USING NULLIF(btrim("create_time"), '')::timestamp;
ALTER TABLE "profit_sharing_return_order" ALTER COLUMN "modify_time" TYPE timestamp USING NULLIF(btrim("modify_time"), '')::timestamp;
ALTER TABLE "sub_merchant" ALTER COLUMN "create_time" TYPE timestamp USING NULLIF(btrim("create_time"), '')::timestamp;
ALTER TABLE "sub_merchant" ALTER COLUMN "modify_time" TYPE timestamp USING NULLIF(btrim("modify_time"), '')::timestamp;
ALTER TABLE "wechat_keyword_rule" ALTER COLUMN "created_at" TYPE timestamp USING NULLIF(btrim("created_at"), '')::timestamp;
ALTER TABLE "wechat_keyword_rule" ALTER COLUMN "updated_at" TYPE timestamp USING NULLIF(btrim("updated_at"), '')::timestamp;
ALTER TABLE "wechat_live" ALTER COLUMN "create_time" TYPE timestamp USING NULLIF(btrim("create_time"), '')::timestamp;
ALTER TABLE "wechat_live_goods" ALTER COLUMN "create_time" TYPE timestamp USING NULLIF(btrim("create_time"), '')::timestamp;
ALTER TABLE "wechat_live_goods" ALTER COLUMN "update_time" TYPE timestamp USING NULLIF(btrim("update_time"), '')::timestamp;
ALTER TABLE "wechat_live_room_goods" ALTER COLUMN "create_time" TYPE timestamp USING NULLIF(btrim("create_time"), '')::timestamp;
ALTER TABLE "wechat_live_room_goods" ALTER COLUMN "update_time" TYPE timestamp USING NULLIF(btrim("update_time"), '')::timestamp;
ALTER TABLE "wechat_menu" ALTER COLUMN "published_at" TYPE timestamp USING NULLIF(btrim("published_at"), '')::timestamp;
ALTER TABLE "wechat_menu" ALTER COLUMN "created_at" TYPE timestamp USING NULLIF(btrim("created_at"), '')::timestamp;
ALTER TABLE "wechat_menu" ALTER COLUMN "updated_at" TYPE timestamp USING NULLIF(btrim("updated_at"), '')::timestamp;

ALTER TABLE "keywords" ALTER COLUMN "date" TYPE date USING NULLIF(btrim("date"), '')::date;
ALTER TABLE "order" ALTER COLUMN "delivery_date" TYPE date USING NULLIF(btrim("delivery_date"), '')::date;
ALTER TABLE "order_cycle" ALTER COLUMN "next_delivery_date" TYPE date USING NULLIF(btrim("next_delivery_date"), '')::date;
ALTER TABLE "order_cycle_card_redeem" ALTER COLUMN "delivery_date" TYPE date USING NULLIF(btrim("delivery_date"), '')::date;
ALTER TABLE "order_cycle_delivery" ALTER COLUMN "delivery_date" TYPE date USING NULLIF(btrim("delivery_date"), '')::date;
ALTER TABLE "statistics_base" ALTER COLUMN "date" TYPE date USING NULLIF(btrim("date"), '')::date;
ALTER TABLE "user" ALTER COLUMN "birthday" TYPE date USING NULLIF(btrim("birthday"), '')::date;

ALTER TABLE "cart" ALTER COLUMN "extra_data" TYPE jsonb USING NULLIF(btrim("extra_data"), '')::jsonb;
ALTER TABLE "cart" ALTER COLUMN "change_price_data" TYPE jsonb USING NULLIF(btrim("change_price_data"), '')::jsonb;
ALTER TABLE "groupon_promotion_detail" ALTER COLUMN "group_price" TYPE jsonb USING NULLIF(btrim("group_price"), '')::jsonb;
ALTER TABLE "groupon_promotion_item" ALTER COLUMN "group_num" TYPE jsonb USING NULLIF(btrim("group_num"), '')::jsonb;
ALTER TABLE "lottery_activity" ALTER COLUMN "decorate_config" TYPE jsonb USING NULLIF(btrim("decorate_config"), '')::jsonb;
ALTER TABLE "lottery_activity" ALTER COLUMN "grid_config" TYPE jsonb USING NULLIF(btrim("grid_config"), '')::jsonb;
ALTER TABLE "order" ALTER COLUMN "change_price_data" TYPE jsonb USING NULLIF(btrim("change_price_data"), '')::jsonb;
ALTER TABLE "order_item" ALTER COLUMN "change_price_data" TYPE jsonb USING NULLIF(btrim("change_price_data"), '')::jsonb;
ALTER TABLE "promotion" ALTER COLUMN "member_scope" TYPE jsonb USING NULLIF(btrim("member_scope"), '')::jsonb;
ALTER TABLE "vendor_product_sku" ALTER COLUMN "sku_attr_json" TYPE jsonb USING NULLIF(btrim("sku_attr_json"), '')::jsonb;

COMMIT;

-- 回滚（示例，按需逐列执行）：
-- ALTER TABLE "cart" ALTER COLUMN "extra_data" TYPE text USING "extra_data"::text;
-- ALTER TABLE "merchant_application" ALTER COLUMN "create_time" TYPE varchar(255)
--   USING to_char("create_time", 'YYYY-MM-DD HH24:MI:SS');
