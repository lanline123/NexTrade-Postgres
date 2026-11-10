-- 2026-11-10 P2 + P3
-- P2：merchant_application 的 abandon_time / delete_time 归位为 text，
--     与同表 audit_time 及全库 49 列 text 口径一致（Rust 实体未映射这两列）。
--     代码侧同步改为 TO_CHAR(NOW(),'YYYY-MM-DD HH24:MI:SS')。
-- P3：为 JSON 包含查询（col::jsonb @> ...）建 GIN 表达式索引。
--     已验证 varchar::jsonb / text::jsonb 在 PG 中可作索引表达式（4/4 通过）。
-- 执行：psql -v ON_ERROR_STOP=1 -f，两段各自事务。

BEGIN;
SET LOCAL lock_timeout='15s';
ALTER TABLE merchant_application
  ALTER COLUMN abandon_time TYPE text USING to_char(abandon_time, 'YYYY-MM-DD HH24:MI:SS'),
  ALTER COLUMN delete_time  TYPE text USING to_char(delete_time,  'YYYY-MM-DD HH24:MI:SS');
COMMIT;

BEGIN;
SET LOCAL lock_timeout='15s';
CREATE INDEX IF NOT EXISTS idx_buyer_show_gin_group_data
  ON public.buyer_show USING gin ((group_data::jsonb));
CREATE INDEX IF NOT EXISTS idx_buyer_show_gin_range_data
  ON public.buyer_show USING gin ((range_data::jsonb));
CREATE INDEX IF NOT EXISTS idx_buyer_show_group_gin_product_group_id
  ON public.buyer_show_group USING gin ((product_group_id::jsonb));
CREATE INDEX IF NOT EXISTS idx_product_group_gin_product_ids
  ON public.product_group USING gin ((product_ids::jsonb));
CREATE INDEX IF NOT EXISTS idx_promotion_gin_range_data
  ON public.promotion USING gin ((range_data::jsonb));
CREATE INDEX IF NOT EXISTS idx_product_promotion_gin_range_data
  ON public.product_promotion USING gin ((range_data::jsonb));
COMMIT;
