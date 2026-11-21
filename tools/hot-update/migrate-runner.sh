#!/usr/bin/env bash
# 迁移执行器（admin 自助热更新 P3，我方新增）——updater 迁移阶段的唯一入口。
#
# 设计依据：~/workspace/hot-update-design/REPORT.md §4.3 / §5.2 / §8：
# - 只执行 job 钉死的 release-manifest.json 快照里列出的迁移文件，顺序 =
#   清单序；执行时禁止再取 latest（红线 1）。
# - 含迁移的 job 必须先 pg_dump 成功才执行第一个文件（红线 6）；磁盘不足
#   或转储失败即中止，一个文件都不跑。
# - 破坏性语句双闸：清单 `destructive` 标志 + 本执行器复扫
#   （classify-destructive.sh）；清单说安全而复扫出破坏性 = 清单不可信，
#   中止；任一待执行文件破坏性且 job 未带 destructiveAck = 中止。
# - 逐文件一事务：整文件走 psql 单会话 --single-transaction +
#   ON_ERROR_STOP（不复用代码内 run_migrations 的朴素分号切分）。含
#   CONCURRENTLY / VACUUM 等不可入事务语句的文件降级为非事务执行，
#   并在结果里如实标注 txMode=none（该文件不保证原子性，验收须知）。
# - 成功即登记 sys_schema_migration（sha256/product_version/applied_by=
#   'hot-update'/job_id）；任一失败 → 该文件事务回滚、整个 job 停在迁移
#   阶段、不换任何面。已在本 job 内先行成功的文件保持已登记状态——
#   **迁移只进不退**（红线 7），不自动回滚 schema。
# - 已登记者跳过；已登记但 sha 与清单不一致 = 文件被改过，中止并告警。
#
# 用法：
#   migrate-runner.sh --job-file <spool/jobs/<id>.json> \
#                     --migrations-dir <已解包的迁移文件目录> \
#                     --backup-dir <pg_dump 落点目录> \
#                     [--result-file <json>] [--dry-run]
#
# 连接：透传标准 PG* 环境变量（PGHOST/PGPORT/PGUSER/PGDATABASE/PGPASSWORD
# 等），二进制可用 PSQL_BIN / PGDUMP_BIN / PGRESTORE_BIN 覆盖；
# HU_DISK_MARGIN_MB = 转储之外要求的额外空闲（默认 512）。
#
# 前置依赖：bash 4+、python3、sha256sum、df。缺任一 → 退出码 5。
#
# 退出码：
#   0 成功（含「无待执行迁移」与 --dry-run 通过全部闸门）
#   2 闸门拒绝（破坏性未确认 / sha 不符 / 清单与复扫不一致 / 包不完整）
#   3 备份门失败（磁盘不足 / pg_dump 失败 / 转储校验不过）
#   4 执行失败（某文件事务回滚，job 停在迁移阶段；或登记写入失败）
#   5 用法/环境错误（参数、依赖、job 文件或登记表不可读）
#
# 结果文件（JSON）：{jobId, productVersion, backupFile, applied[],
# skipped[], failed, stage, message, dryRun}，供 updater 与演练脚本断言。
# stdout 逐行进度（MIGRATE ... / GATE ... / BACKUP ...），由 updater 收进
# job 日志；凭据永不出现在任何输出里（连接串不回显）。

set -u -o pipefail

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
# shellcheck source=classify-destructive.sh
. "$SCRIPT_DIR/classify-destructive.sh"

PSQL_BIN=${PSQL_BIN:-psql}
PGDUMP_BIN=${PGDUMP_BIN:-pg_dump}
PGRESTORE_BIN=${PGRESTORE_BIN:-pg_restore}
HU_DISK_MARGIN_MB=${HU_DISK_MARGIN_MB:-512}

JOB_FILE="" MIGRATIONS_DIR="" BACKUP_DIR="" RESULT_FILE="" DRY_RUN=0
while [ $# -gt 0 ]; do
  case "$1" in
    --job-file) JOB_FILE=$2; shift 2 ;;
    --migrations-dir) MIGRATIONS_DIR=$2; shift 2 ;;
    --backup-dir) BACKUP_DIR=$2; shift 2 ;;
    --result-file) RESULT_FILE=$2; shift 2 ;;
    --dry-run) DRY_RUN=1; shift ;;
    *) echo "ERROR unknown arg: $1" >&2; exit 5 ;;
  esac
done
[ -n "$JOB_FILE" ] && [ -n "$MIGRATIONS_DIR" ] && [ -n "$BACKUP_DIR" ] || {
  echo "ERROR usage: migrate-runner.sh --job-file F --migrations-dir D --backup-dir B [--result-file F] [--dry-run]" >&2; exit 5; }
[ -f "$JOB_FILE" ] || { echo "ERROR job file not found: $JOB_FILE" >&2; exit 5; }
[ -d "$MIGRATIONS_DIR" ] || { echo "ERROR migrations dir not found: $MIGRATIONS_DIR" >&2; exit 5; }
for dep in python3 sha256sum df "$PSQL_BIN" "$PGDUMP_BIN"; do
  command -v "$dep" >/dev/null 2>&1 || { echo "ERROR missing dependency: $dep" >&2; exit 5; }
done
[ -n "$RESULT_FILE" ] || RESULT_FILE="$(dirname "$JOB_FILE")/../status/$(basename "$JOB_FILE" .json).migrate-result.json"

TMPD=$(mktemp -d)
trap 'rm -rf "$TMPD"' EXIT

# ——— 解析 job 文件（清单快照是唯一事实源）———
# 产出：meta.env（JOB_ID/PRODUCT_VERSION/DESTRUCTIVE_ACK/INCLUDE_MIGRATIONS）
# 与 migrations.tsv（file \t sha256 \t destructive \t requiresBackendRestart）
python3 - "$JOB_FILE" "$TMPD" <<'PYEOF'
import json, sys
job_path, tmpd = sys.argv[1], sys.argv[2]
with open(job_path, encoding="utf-8") as fh:
    job = json.load(fh)
manifest = job.get("manifest") or {}
def shq(v):
    return "'" + str(v).replace("'", "'\\''") + "'"
with open(f"{tmpd}/meta.env", "w", encoding="utf-8") as fh:
    fh.write(f"JOB_ID={shq(job.get('jobId',''))}\n")
    fh.write(f"PRODUCT_VERSION={shq(job.get('productVersion') or manifest.get('productVersion',''))}\n")
    fh.write(f"DESTRUCTIVE_ACK={shq('true' if job.get('destructiveAck') else 'false')}\n")
    fh.write(f"INCLUDE_MIGRATIONS={shq('true' if job.get('includeMigrations') else 'false')}\n")
with open(f"{tmpd}/migrations.tsv", "w", encoding="utf-8") as fh:
    for mig in manifest.get("migrations") or []:
        fh.write("\t".join([
            str(mig.get("file", "")),
            str(mig.get("sha256", "")).lower(),
            "true" if mig.get("destructive") else "false",
            "true" if mig.get("requiresBackendRestart") else "false",
        ]) + "\n")
PYEOF
[ $? -eq 0 ] || { echo "ERROR job file parse failed: $JOB_FILE" >&2; exit 5; }
# shellcheck disable=SC1091
. "$TMPD/meta.env"
[ -n "$JOB_ID" ] || { echo "ERROR job file missing jobId" >&2; exit 5; }

psql_q() { # 只读/登记类小查询：-t -A 输出
  "$PSQL_BIN" -X -q -t -A -v ON_ERROR_STOP=1 -c "$1"
}

SKIPPED_FILES=()
APPLIED_FILES=()
write_result() { # backup_file failed_file stage message
  python3 - "$RESULT_FILE" "$JOB_ID" "$PRODUCT_VERSION" "$1" "$2" "$3" "$4" \
    "$DRY_RUN" "${SKIPPED_FILES[*]:-}" "${APPLIED_FILES[*]:-}" <<'PYEOF'
import json, sys, time
(_, path, job_id, version, backup, failed, stage, message, dry,
 skipped, applied) = sys.argv
doc = {
    "jobId": job_id,
    "productVersion": version,
    "backupFile": backup or None,
    "applied": applied.split() if applied else [],
    "skipped": skipped.split() if skipped else [],
    "failed": failed or None,
    "stage": stage,
    "message": message,
    "dryRun": dry == "1",
    "finishedAt": int(time.time()),
}
with open(path, "w", encoding="utf-8") as fh:
    json.dump(doc, fh, ensure_ascii=False, indent=2)
PYEOF
}

# ——— 读登记表快照 ———
if ! psql_q "SELECT filename, sha256 FROM sys_schema_migration" > "$TMPD/registered.tsv" 2>"$TMPD/registered.err"; then
  echo "ERROR cannot read sys_schema_migration (baseline missing?): $(tail -1 "$TMPD/registered.err")" >&2
  exit 5
fi
declare -A REGISTERED=()
while IFS=$'\t' read -r rf rs; do
  [ -n "$rf" ] && REGISTERED["$rf"]=$(printf '%s' "$rs" | tr '[:upper:]' '[:lower:]')
done < "$TMPD/registered.tsv"

# ——— 计算待执行集 + 完整性/sha 闸 ———
PENDING_FILES=()
declare -A PENDING_SHA=() PENDING_DESTRUCTIVE=()
while IFS=$'\t' read -r file sha destructive _restart; do
  [ -n "$file" ] || continue
  if [ -n "${REGISTERED[$file]:-}" ]; then
    if [ "${REGISTERED[$file]}" != "$sha" ]; then
      echo "GATE sha_mismatch_registered file=$file registered=${REGISTERED[$file]} manifest=$sha" >&2
      write_result "" "" "gate" "已登记迁移 $file 的 sha 与清单不一致（文件被改过），中止"
      exit 2
    fi
    SKIPPED_FILES+=("$file")
    continue
  fi
  local_path="$MIGRATIONS_DIR/$file"
  if [ ! -f "$local_path" ]; then
    echo "GATE missing_file file=$file" >&2
    write_result "" "" "gate" "更新包缺少待执行迁移文件 $file，中止"
    exit 2
  fi
  actual=$(sha256sum "$local_path" | awk '{print $1}')
  if [ "$actual" != "$sha" ]; then
    echo "GATE sha_mismatch_file file=$file actual=$actual manifest=$sha" >&2
    write_result "" "" "gate" "迁移文件 $file 的 sha256 与清单不一致，中止"
    exit 2
  fi
  PENDING_FILES+=("$file")
  PENDING_SHA["$file"]=$sha
  PENDING_DESTRUCTIVE["$file"]=$destructive
done < "$TMPD/migrations.tsv"

if [ "${#PENDING_FILES[@]}" -eq 0 ]; then
  echo "MIGRATE nothing pending (all registered)"
  write_result "" "" "done" "无待执行迁移"
  exit 0
fi
echo "MIGRATE pending: ${PENDING_FILES[*]}"

# ——— 破坏性双闸 ———
for file in "${PENDING_FILES[@]}"; do
  scan_out=$(hu_classify_destructive "$MIGRATIONS_DIR/$file")
  scan_rc=$?
  if [ "$scan_rc" = "3" ]; then
    echo "SCAN destructive file=$file: $scan_out"
    if [ "${PENDING_DESTRUCTIVE[$file]}" != "true" ]; then
      echo "GATE manifest_scan_disagree file=$file" >&2
      write_result "" "" "gate" "迁移 $file 复扫出破坏性语句但清单未标注，清单不可信，中止"
      exit 2
    fi
  elif [ "$scan_rc" != "0" ]; then
    echo "ERROR classifier failed on $file" >&2
    write_result "" "" "gate" "破坏性复扫失败：$file"
    exit 2
  fi
  if { [ "${PENDING_DESTRUCTIVE[$file]}" = "true" ] || [ "$scan_rc" = "3" ]; } \
     && [ "$DESTRUCTIVE_ACK" != "true" ]; then
    echo "GATE destructive_no_ack file=$file" >&2
    write_result "" "" "gate" "迁移 $file 含破坏性语句且 job 未带破坏性确认（destructiveAck），中止"
    exit 2
  fi
done

if [ "$DRY_RUN" = "1" ]; then
  echo "MIGRATE dry-run: gates passed, ${#PENDING_FILES[@]} file(s) would run"
  write_result "" "" "dry_run" "演练模式：闸门全过，未执行"
  exit 0
fi

# ——— 备份门：磁盘预检 + pg_dump + 转储可读性校验 ———
mkdir -p "$BACKUP_DIR" || { echo "ERROR cannot create backup dir: $BACKUP_DIR" >&2; exit 3; }
db_size=$(psql_q "SELECT pg_database_size(current_database())" 2>/dev/null | tr -d '[:space:]')
case "$db_size" in ''|*[!0-9]*) db_size=0 ;; esac
free_kb=$(df -P "$BACKUP_DIR" | awk 'NR==2 {print $4}')
need_kb=$(( db_size / 1024 + HU_DISK_MARGIN_MB * 1024 ))
if [ "${free_kb:-0}" -lt "$need_kb" ]; then
  echo "GATE disk_low free_kb=${free_kb:-0} need_kb=$need_kb" >&2
  write_result "" "" "backup" "备份目录磁盘空间不足（空闲 ${free_kb:-0}KB < 需要 ${need_kb}KB），中止"
  exit 3
fi
ts=$(date +%Y%m%d%H%M%S)
BACKUP_FILE="$BACKUP_DIR/pre-migrate-${JOB_ID:0:8}-$ts.dump"
echo "BACKUP start -> $BACKUP_FILE"
if ! "$PGDUMP_BIN" -Fc -f "$BACKUP_FILE" 2>"$TMPD/pgdump.err"; then
  echo "ERROR pg_dump failed: $(tail -1 "$TMPD/pgdump.err")" >&2
  rm -f "$BACKUP_FILE"
  write_result "" "" "backup" "pg_dump 失败，中止（未执行任何迁移）"
  exit 3
fi
backup_size=$(stat -c %s "$BACKUP_FILE" 2>/dev/null || echo 0)
if [ "$backup_size" -lt 1024 ]; then
  echo "ERROR pg_dump output suspiciously small: ${backup_size}B" >&2
  write_result "" "" "backup" "pg_dump 产物异常小（${backup_size}B），中止"
  exit 3
fi
if command -v "$PGRESTORE_BIN" >/dev/null 2>&1; then
  if ! "$PGRESTORE_BIN" -l "$BACKUP_FILE" >/dev/null 2>&1; then
    echo "ERROR pg_restore cannot read dump" >&2
    write_result "$BACKUP_FILE" "" "backup" "pg_dump 产物无法被 pg_restore 读取，中止"
    exit 3
  fi
fi
echo "BACKUP ok size=${backup_size}B"

# ——— 逐文件执行（清单序）———
sql_escape() { printf '%s' "$1" | sed "s/'/''/g"; }
for file in "${PENDING_FILES[@]}"; do
  local_path="$MIGRATIONS_DIR/$file"
  # 执行前复验 sha（防下载后被替换）
  actual=$(sha256sum "$local_path" | awk '{print $1}')
  if [ "$actual" != "${PENDING_SHA[$file]}" ]; then
    echo "GATE sha_recheck_mismatch file=$file" >&2
    write_result "$BACKUP_FILE" "$file" "gate" "执行前复验 sha 不符：$file"
    exit 2
  fi
  tx_args=(--single-transaction)
  tx_mode="single-transaction"
  if grep -qiE 'CONCURRENTLY|^[[:space:]]*VACUUM|DISCARD[[:space:]]+ALL' "$local_path"; then
    tx_args=()
    tx_mode="none"
    echo "MIGRATE warn file=$file contains non-transactional statements, txMode=none"
  fi
  echo "MIGRATE run file=$file txMode=$tx_mode"
  if ! "$PSQL_BIN" -X -q -v ON_ERROR_STOP=1 "${tx_args[@]}" -f "$local_path" \
       >"$TMPD/run.out" 2>"$TMPD/run.err"; then
    echo "ERROR migration failed file=$file: $(tail -2 "$TMPD/run.err")" >&2
    write_result "$BACKUP_FILE" "$file" "migrating" \
      "迁移 $file 执行失败已回滚（txMode=$tx_mode）：$(tail -1 "$TMPD/run.err")；job 停在迁移阶段，未换任何面；此前已成功文件保持登记（迁移只进不退）"
    exit 4
  fi
  now_s=$(date +%s)
  reg_sql="INSERT INTO sys_schema_migration (filename, sha256, product_version, applied_at, applied_by, job_id) VALUES ('$(sql_escape "$file")', '$(sql_escape "${PENDING_SHA[$file]}")', '$(sql_escape "$PRODUCT_VERSION")', $now_s, 'hot-update', '$(sql_escape "$JOB_ID")') ON CONFLICT (filename) DO NOTHING"
  if ! psql_q "$reg_sql" >/dev/null 2>"$TMPD/reg.err"; then
    echo "ERROR registration failed file=$file: $(tail -1 "$TMPD/reg.err")" >&2
    write_result "$BACKUP_FILE" "$file" "register_failed" \
      "迁移 $file 已执行成功但登记写入失败——库已前进、登记未跟上，严禁重试整个 job，须人工核对 sys_schema_migration 后补登记"
    exit 4
  fi
  APPLIED_FILES+=("$file")
  echo "MIGRATE ok file=$file"
done

echo "MIGRATE done applied=${#APPLIED_FILES[@]} backup=$BACKUP_FILE"
write_result "$BACKUP_FILE" "" "done" "迁移全部完成（${#APPLIED_FILES[@]} 个文件），备份：$BACKUP_FILE"
exit 0
