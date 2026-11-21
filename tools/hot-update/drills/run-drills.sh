#!/usr/bin/env bash
# 热更新 P3 灾难路径演练（桩件版，我方新增）。
#
# 目的：在**不接真实 PG / 真实 systemd / 生产目录**的前提下，把 P3 的
# 关键灾难路径全部真跑一遍——迁移中途失败、破坏性闸门、sha 不符、
# pg_dump 失败、后端新件起不来自动回滚。psql/pg_dump/systemctl 用桩
# 件模拟（状态落沙箱目录），版本端点用一次性 python HTTP 服务模拟。
#
# 真实 PG 上的克隆环境演练（真事务回滚、真 pg_dump/restore）见同目录
# CLONE-ENV-DRILL.md，是 P3 上生产前的必备验收项；本脚本通过 ≠ 可跳过
# 克隆环境演练。
#
# 用法：bash run-drills.sh [沙箱目录]（缺省 mktemp -d，结束保留并打印）
# 退出码：0 = 全部用例 PASS；1 = 至少一个 FAIL。

set -u -o pipefail
TOOLS_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
SB=${1:-$(mktemp -d)}
mkdir -p "$SB"/{stubs,state,migs,jobs,backups,bin,assets,status,spool/jobs,spool/status}
STATE="$SB/state"
PASS=0 FAIL=0
note() { echo "── $*"; }
ok()   { PASS=$((PASS+1)); echo "PASS $1"; }
bad()  { FAIL=$((FAIL+1)); echo "FAIL $1: $2"; }
check(){ # name condition...
  local name=$1; shift
  if "$@" >/dev/null 2>&1; then ok "$name"; else bad "$name" "condition failed: $*"; fi
}

# ═══ 桩件 ═══
cat > "$SB/stubs/psql" <<'EOF'
#!/usr/bin/env bash
# 桩 psql：状态 = $DRILL_STATE/registered.tsv（filename \t sha）
echo "psql $*" >> "$DRILL_STATE/psql.log"
sql="" file=""
while [ $# -gt 0 ]; do
  case "$1" in
    -c) sql=$2; shift 2 ;;
    -f) file=$2; shift 2 ;;
    *) shift ;;
  esac
done
if [ -n "$sql" ]; then
  case "$sql" in
    *"INSERT INTO sys_schema_migration"*)
      python3 - "$DRILL_STATE/registered.tsv" "$sql" <<'PY'
import re, sys
path, sql = sys.argv[1], sys.argv[2]
m = re.search(r"VALUES \('([^']*)',\s*'([^']*)'", sql)
if m:
    with open(path, "a", encoding="utf-8") as fh:
        fh.write(f"{m.group(1)}\t{m.group(2)}\n")
PY
      exit 0 ;;
    *"FROM sys_schema_migration"*)
      [ -f "$DRILL_STATE/registered.tsv" ] && cat "$DRILL_STATE/registered.tsv"
      exit 0 ;;
    *"pg_database_size"*)
      echo 1048576; exit 0 ;;
    *) exit 0 ;;
  esac
fi
if [ -n "$file" ]; then
  if [ "$(basename "$file")" = "${FAKE_PSQL_FAIL_ON:-__none__}" ]; then
    echo "ERROR: simulated SQL failure in $file" >&2
    exit 3
  fi
  exit 0
fi
exit 0
EOF
cat > "$SB/stubs/pg_dump" <<'EOF'
#!/usr/bin/env bash
[ "${FAKE_PGDUMP_FAIL:-0}" = "1" ] && { echo "simulated pg_dump failure" >&2; exit 1; }
out=""
while [ $# -gt 0 ]; do case "$1" in -f) out=$2; shift 2 ;; *) shift ;; esac; done
[ -n "$out" ] || exit 1
head -c 4096 /dev/zero | tr '\0' 'D' > "$out"
exit 0
EOF
cat > "$SB/stubs/pg_restore" <<'EOF'
#!/usr/bin/env bash
# 仅支持 -l 校验：文件存在且非空即视为可读
last=""
for a in "$@"; do last=$a; done
[ -s "$last" ] && exit 0 || exit 1
EOF
cat > "$SB/stubs/systemctl" <<'EOF'
#!/usr/bin/env bash
# 桩 systemctl：restart 后按当前已装二进制内的 PV=/GIT_SHA= 行更新 serving.json；
# 已装二进制 sha 命中 $DRILL_STATE/bad_sha 时模拟新件起不来（unit down）。
cmd=${1:-}
case "$cmd" in
  is-active)
    [ -f "$DRILL_STATE/unit_down" ] && exit 3
    exit 0 ;;
  restart)
    bin=${HU_BACKEND_BIN:?}
    sha=$(sha256sum "$bin" | awk '{print $1}')
    if [ -f "$DRILL_STATE/bad_sha" ] && [ "$sha" = "$(cat "$DRILL_STATE/bad_sha")" ]; then
      touch "$DRILL_STATE/unit_down"
      rm -f "$DRILL_STATE/serving.json"
      exit 1
    fi
    rm -f "$DRILL_STATE/unit_down"
    pv=$(grep -a '^PV=' "$bin" | head -1 | cut -d= -f2)
    gs=$(grep -a '^GIT_SHA=' "$bin" | head -1 | cut -d= -f2)
    python3 - "$DRILL_STATE/serving.json" "$pv" "$gs" <<'PY'
import json, sys
with open(sys.argv[1], "w") as fh:
    json.dump({"productVersion": sys.argv[2], "gitSha": sys.argv[3]}, fh)
PY
    exit 0 ;;
  *) exit 0 ;;
esac
EOF
chmod +x "$SB/stubs/"*
export DRILL_STATE="$STATE"

# ═══ 版本端点模拟服务 ═══
cat > "$SB/serve.py" <<'EOF'
import json, os
from http.server import BaseHTTPRequestHandler, HTTPServer
STATE = os.environ["DRILL_STATE"]
class H(BaseHTTPRequestHandler):
    def do_GET(self):
        p = os.path.join(STATE, "serving.json")
        if not os.path.exists(p):
            self.send_response(500); self.end_headers(); return
        if self.path.startswith("/api/version"):
            data = json.load(open(p))
            body = json.dumps({"code": 0, "data": data}).encode()
        elif self.path.startswith("/api/common/config/themeSettings"):
            body = b'{"code":0,"data":{}}'
        else:
            self.send_response(404); self.end_headers(); return
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.end_headers()
        self.wfile.write(body)
    def log_message(self, *a):
        pass
HTTPServer(("127.0.0.1", 18099), H).serve_forever()
EOF
python3 "$SB/serve.py" &
SERVER_PID=$!
trap 'kill $SERVER_PID 2>/dev/null; echo; echo "沙箱保留在: $SB"; echo "演练结果: PASS=$PASS FAIL=$FAIL"' EXIT
sleep 0.5

# ═══ 迁移夹具与 job 构造 ═══
printf 'CREATE TABLE d1 (id int);\n'                       > "$SB/migs/901_d1_ok.sql"
printf 'CREATE TABLE d2 (id int);\nTHIS IS BROKEN SQL;\n'  > "$SB/migs/902_d2_fail.sql"
printf 'CREATE TABLE d3 (id int);\n'                       > "$SB/migs/903_d3_ok.sql"
printf 'DROP TABLE legacy_x;\n'                            > "$SB/migs/904_d4_drop.sql"
printf 'DROP TABLE sneaky;\n'                              > "$SB/migs/905_d5_sneaky.sql"

make_job() { # job_id ack include_migrations migrations_json_array
  python3 - "$SB/spool/jobs/$1.json" "$1" "$2" "$3" "$4" <<'PY'
import json, sys
path, jid, ack, inc, migs = sys.argv[1:6]
job = {
    "jobId": jid, "productVersion": "0.0.3", "releaseTag": "v0.0.3",
    "repo": "lanline123/NexTrade-Postgres", "createdAt": 1,
    "operator": "drill", "surfaces": ["backend"], "schemaReady": True,
    "manifestSha256": "0" * 64,
    "destructiveAck": ack == "true",
    "includeMigrations": inc == "true",
    "manifest": {"productVersion": "0.0.3", "gitCommit": "deadbeef",
                 "surfaces": {}, "migrations": json.loads(migs), "compat": {}},
}
with open(path, "w", encoding="utf-8") as fh:
    json.dump(job, fh)
PY
}
mig_json() { # file destructive(0/1) [sha_override]
  python3 - "$SB/migs/$1" "$1" "$2" "${3:-}" <<'PY'
import hashlib, json, sys
path, name, destr, override = sys.argv[1:5]
sha = override or hashlib.sha256(open(path, "rb").read()).hexdigest()
print(json.dumps({"file": name, "sha256": sha, "summary": "drill",
                  "changes": [], "destructive": destr == "1",
                  "requiresBackendRestart": True}))
PY
}
run_runner() { # job_id [env assignments...]
  local jid=$1; shift
  env PSQL_BIN="$SB/stubs/psql" PGDUMP_BIN="$SB/stubs/pg_dump" \
      PGRESTORE_BIN="$SB/stubs/pg_restore" HU_DISK_MARGIN_MB=1 "$@" \
    bash "$TOOLS_DIR/migrate-runner.sh" \
      --job-file "$SB/spool/jobs/$jid.json" \
      --migrations-dir "$SB/migs" \
      --backup-dir "$SB/backups" \
      --result-file "$SB/status/$jid.migrate-result.json"
}
reset_state() { rm -f "$STATE/registered.tsv" "$STATE/psql.log"; rm -f "$SB"/backups/*.dump "$SB"/status/*.json; }

# ═══ D1 迁移中途失败 ═══
note "D1 迁移中途失败：第 2 个文件失败 → 停在迁移阶段，第 1 个保持登记，第 3 个不跑"
reset_state
make_job job-d1 false true "[$(mig_json 901_d1_ok.sql 0),$(mig_json 902_d2_fail.sql 0),$(mig_json 903_d3_ok.sql 0)]"
FAKE_PSQL_FAIL_ON=902_d2_fail.sql run_runner job-d1; rc=$?
[ "$rc" = "4" ] && ok "D1 退出码 4" || bad "D1 退出码" "rc=$rc want 4"
check "D1 第 1 个已登记" grep -q '^901_d1_ok.sql' "$STATE/registered.tsv"
check "D1 第 3 个未登记" bash -c "! grep -q '903_d3_ok' '$STATE/registered.tsv'"
check "D1 结果文件 failed=902" python3 -c "
import json; d=json.load(open('$SB/status/job-d1.migrate-result.json'))
assert d['failed']=='902_d2_fail.sql' and d['applied']==['901_d1_ok.sql'] and d['backupFile']"
check "D1 备份已产生" bash -c "ls '$SB'/backups/pre-migrate-*.dump >/dev/null"

# ═══ D2 破坏性无确认 ═══
note "D2 破坏性迁移未带 destructiveAck → 闸门拒绝，且不浪费备份"
reset_state
make_job job-d2 false true "[$(mig_json 904_d4_drop.sql 1)]"
run_runner job-d2; rc=$?
[ "$rc" = "2" ] && ok "D2 退出码 2" || bad "D2 退出码" "rc=$rc want 2"
check "D2 未登记" bash -c "! grep -q '904' '$STATE/registered.tsv' 2>/dev/null"
check "D2 未产生备份" bash -c "! ls '$SB'/backups/*.dump >/dev/null 2>&1"

# ═══ D3 sha 不符 ═══
note "D3 清单 sha 与文件不符 → 闸门拒绝"
reset_state
make_job job-d3 false true "[$(mig_json 901_d1_ok.sql 0 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa')]"
run_runner job-d3; rc=$?
[ "$rc" = "2" ] && ok "D3 退出码 2" || bad "D3 退出码" "rc=$rc want 2"
check "D3 未登记" bash -c "! grep -q '901' '$STATE/registered.tsv' 2>/dev/null"

# ═══ D4 pg_dump 失败 ═══
note "D4 pg_dump 失败 → 备份门中止，一个迁移都不跑"
reset_state
make_job job-d4 false true "[$(mig_json 901_d1_ok.sql 0)]"
FAKE_PGDUMP_FAIL=1 run_runner job-d4; rc=$?
[ "$rc" = "3" ] && ok "D4 退出码 3" || bad "D4 退出码" "rc=$rc want 3"
check "D4 未登记" bash -c "! grep -q '901' '$STATE/registered.tsv' 2>/dev/null"

# ═══ D5 清单与复扫不一致 ═══
note "D5 清单标注非破坏性但原文含 DROP → 清单不可信，中止（即使已确认）"
reset_state
make_job job-d5 true true "[$(mig_json 905_d5_sneaky.sql 0)]"
run_runner job-d5; rc=$?
[ "$rc" = "2" ] && ok "D5 退出码 2" || bad "D5 退出码" "rc=$rc want 2"
check "D5 未登记" bash -c "! grep -q '905' '$STATE/registered.tsv' 2>/dev/null"

# ═══ D6 正常路径 + 幂等重跑 ═══
note "D6 正常路径全过；同 job 重跑 → 无待执行、直接成功"
reset_state
make_job job-d6 false true "[$(mig_json 901_d1_ok.sql 0),$(mig_json 903_d3_ok.sql 0)]"
run_runner job-d6; rc=$?
[ "$rc" = "0" ] && ok "D6 退出码 0" || bad "D6 退出码" "rc=$rc want 0"
check "D6 两文件已登记" bash -c "grep -q '901' '$STATE/registered.tsv' && grep -q '903' '$STATE/registered.tsv'"
run_runner job-d6; rc=$?
[ "$rc" = "0" ] && ok "D6 重跑退出码 0" || bad "D6 重跑退出码" "rc=$rc want 0"
check "D6 重跑 applied 为空" python3 -c "
import json; d=json.load(open('$SB/status/job-d6.migrate-result.json'))
assert d['applied']==[] and len(d['skipped'])==2"

# ═══ 后端换版夹具 ═══
make_bin() { # path pv git_sha
  { printf '\177ELF\n'; printf 'PV=%s\nGIT_SHA=%s\n' "$2" "$3"; head -c 2048 /dev/zero | tr '\0' 'B'; } > "$1"
}
OLD_GIT=1111111111111111111111111111111111111111
NEW_GIT=2222222222222222222222222222222222222222
BAD_GIT=3333333333333333333333333333333333333333
make_bin "$SB/bin/nextrade" 0.0.2 "$OLD_GIT"
make_bin "$SB/assets/nextrade-new" 0.0.3 "$NEW_GIT"
make_bin "$SB/assets/nextrade-bad" 0.0.3 "$BAD_GIT"
# 让模拟服务先跑起旧版本
HU_BACKEND_BIN="$SB/bin/nextrade" "$SB/stubs/systemctl" restart nextrade
export HU_BACKEND_BIN="$SB/bin/nextrade" HU_BACKEND_UNIT=nextrade
export HU_SYSTEMCTL="$SB/stubs/systemctl"
export HU_BACKEND_HEALTH_URLS="http://127.0.0.1:18099/api/version"
export HU_BACKEND_LEGACY_URLS="http://127.0.0.1:18099/api/common/config/themeSettings"
export HU_HEALTH_TIMEOUT_SECS=8 HU_HEALTH_INTERVAL_SECS=1

# ═══ D7 后端正常换版 ═══
note "D7 后端正常换版：新件装上、版本端点确认新 gitSha、回滚件留存"
new_sha=$(sha256sum "$SB/assets/nextrade-new" | awk '{print $1}')
bash "$TOOLS_DIR/backend-swap.sh" --asset "$SB/assets/nextrade-new" \
  --expect-sha "$new_sha" --expect-version 0.0.3 --expect-git-sha "$NEW_GIT" \
  --job-id job-d7d7d7d7; rc=$?
[ "$rc" = "0" ] && ok "D7 退出码 0" || bad "D7 退出码" "rc=$rc want 0"
check "D7 已装新件" bash -c "[ \"\$(sha256sum '$SB/bin/nextrade' | awk '{print \$1}')\" = '$new_sha' ]"
check "D7 回滚件存在" bash -c "ls '$SB'/bin/nextrade_pre_hotupdate_job-d7d7* >/dev/null"

# ═══ D8 后端新件起不来 → 自动回滚 ═══
note "D8 后端新件起不来（重启即崩）→ 健康检查失败 → 自动回滚旧件并严验旧 gitSha"
# 先把 D7 的新件当作「现行版本」基线：回滚目标 = D7 装上的 0.0.3
bad_sha=$(sha256sum "$SB/assets/nextrade-bad" | awk '{print $1}')
echo "$bad_sha" > "$STATE/bad_sha"
bash "$TOOLS_DIR/backend-swap.sh" --asset "$SB/assets/nextrade-bad" \
  --expect-sha "$bad_sha" --expect-version 0.0.3 --expect-git-sha "$BAD_GIT" \
  --job-id job-d8d8d8d8; rc=$?
[ "$rc" = "4" ] && ok "D8 退出码 4（已回滚）" || bad "D8 退出码" "rc=$rc want 4"
check "D8 已装回 D7 新件" bash -c "[ \"\$(sha256sum '$SB/bin/nextrade' | awk '{print \$1}')\" = '$new_sha' ]"
check "D8 服务在跑且版本=NEW_GIT" python3 -c "
import json; d=json.load(open('$STATE/serving.json')); assert d['gitSha']=='$NEW_GIT'"
rm -f "$STATE/bad_sha"

# ═══ D9 状态机扩展库集成 ═══
note "D9 updater-p3 阶段库：迁移失败翻译为 migration_failed 终态；静态 job 跳过迁移阶段"
reset_state
make_job job-d9 false true "[$(mig_json 901_d1_ok.sql 0),$(mig_json 902_d2_fail.sql 0)]"
mkdir -p "$SB/work-d9/migrations" && cp "$SB"/migs/*.sql "$SB/work-d9/migrations/"
(
  . "$TOOLS_DIR/updater-p3.sh"
  export PSQL_BIN="$SB/stubs/psql" PGDUMP_BIN="$SB/stubs/pg_dump" \
         PGRESTORE_BIN="$SB/stubs/pg_restore" HU_DISK_MARGIN_MB=1 \
         FAKE_PSQL_FAIL_ON=902_d2_fail.sql
  hu3_migrating_phase "$SB/spool/jobs/job-d9.json" "$SB/work-d9" "$SB/backups"
) ; rc=$?
[ "$rc" = "4" ] && ok "D9 阶段返回 4" || bad "D9 阶段返回" "rc=$rc want 4"
check "D9 状态文件=migration_failed" python3 -c "
import json; d=json.load(open('$SB/spool/status/job-d9.json'))
assert d['status']=='migration_failed', d"
make_job job-d9b false false "[]"
python3 - <<PY
import json
p = "$SB/spool/jobs/job-d9b.json"
d = json.load(open(p)); d["surfaces"] = ["admin"]; json.dump(d, open(p, "w"))
PY
(
  . "$TOOLS_DIR/updater-p3.sh"
  hu3_migrating_phase "$SB/spool/jobs/job-d9b.json" "$SB/work-d9" "$SB/backups"
) ; rc=$?
[ "$rc" = "0" ] && ok "D9 静态 job 跳过迁移阶段" || bad "D9 静态 job" "rc=$rc want 0"

echo
echo "══ 演练汇总：PASS=$PASS FAIL=$FAIL ══"
[ "$FAIL" = "0" ]
