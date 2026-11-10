-- 2026-11-10: restore MySQL UNIQUE KEY uk_owner (owner_type, owner_id) on shop_deposit.
-- Lost during migration (PG kept it as a plain index, unique=false).
-- Rust lock_or_create_account_in_tx does SELECT ... FOR UPDATE -> INSERT -> SELECT,
-- which cannot serialize on a missing row; without UNIQUE the race can create
-- duplicate deposit accounts (Java relied on DuplicateKeyException).
-- CONCURRENTLY: does not block reads/writes; table was empty when executed.
CREATE UNIQUE INDEX CONCURRENTLY IF NOT EXISTS uk_shop_deposit_owner
    ON shop_deposit (owner_type, owner_id);

-- The old non-unique index on the same columns is now redundant.
DROP INDEX CONCURRENTLY IF EXISTS idx_shop_deposit_uk_owner;
