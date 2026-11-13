#!/bin/bash
# 生产 PG 主键导出（在服务器执行；执行通道见 HANDOVER §4.1）
# 用法：python scratch/run_shell.py probe-pg-pks.sh <tag>
{
PSQL="/www/server/pgsql/bin/psql -h 127.0.0.1 -U NexTrade -d nextrade -X -A -t -F |"
ok=1
if ! $PSQL -c "
SELECT c.relname AS tbl, a.attname AS pkcol, format_type(a.atttypid, a.atttypmod) AS pktype
FROM pg_class c
JOIN pg_index i ON c.oid = i.indrelid AND i.indisprimary
JOIN pg_attribute a ON a.attrelid = c.oid AND a.attnum = ANY(i.indkey)
WHERE c.relnamespace = 'public'::regnamespace AND c.relkind = 'r'
ORDER BY c.relname, a.attnum;
" > /www/backup/nextrade-pks.tsv; then
    ok=0
    echo "=== psql FAILED: nextrade-pks.tsv may be incomplete (stderr above) ==="
fi
echo "=== pk rows ==="
wc -l /www/backup/nextrade-pks.tsv
echo "=== sample ==="
head -8 /www/backup/nextrade-pks.tsv
echo "=== collect_shop duplicates check ==="
$PSQL -c "SELECT count(*) AS total, count(DISTINCT (user_id, shop_id)) AS distinct_pairs FROM collect_shop;" 2>&1 | head -3
echo "=== collect_shop indexes ==="
$PSQL -c "SELECT indexname, indexdef FROM pg_indexes WHERE schemaname='public' AND tablename='collect_shop';" 2>&1 | head -10
if [ "$ok" -eq 1 ]; then
    echo "PGOPS_DONE"
else
    echo "PGOPS_FAILED"
fi
} 2>&1
