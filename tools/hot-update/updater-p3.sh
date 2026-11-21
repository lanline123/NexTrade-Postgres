#!/usr/bin/env bash
# updater P3 阶段库（admin 自助热更新 P3，我方新增）——被主 updater source。
#
# 设计依据：~/workspace/hot-update-design/REPORT.md §4.3 / §5.2 / §6-P3。
# 对齐基线：P1+P2 暂存快照中的主 updater
# `tools/hot-update/updater/nextrade-updater.sh`（487 行，write_status /
# parse_job / process_job / rollback_all 结构与 updater.conf 配置面）。
# P1+P2 线若再改主 updater 结构，合并时按本文件头的插入点逐条复核。
#
# ══ P3 状态机（在 P2 状态机上扩展）══
#
#   queued → downloading → verifying → backing_up
#     → migrating        （P3 新增；job 含 backend 时进入，迁移先于一切换面）
#     → swapping         （后端先换并严验版本，其余面随后；任一面失败
#                          全 job 回滚已换面——含后端，schema 不退）
#     → health_checking → done
#   终态：done / failed / migration_failed（P3 新增：停在迁移阶段、
#          未换任何面、待人工）/ rolled_back / rollback_failed
#
# ══ 合流补丁（对 nextrade-updater.sh 的五处改动，合并时逐条落）══
#
# 补丁 1 · parse_job：允许 backend 面——删 `if s == "backend": die(...)`
#   一段，allowed 集合加 "backend"。其余校验（schemaReady、manifest
#   资产齐全）对 backend 天然适用（make_manifest 的 backend 资产 =
#   裸二进制 file/sha256/stamp，stamp = 完整 git sha）。
# 补丁 2 · 下载循环：backend 资产是裸二进制，**跳过 tar 解包与包内校验**
#   （`case "$s" in backend) : ;;`），文件留 $WORK_DIR/<file> 供换版用；
#   另在含 backend 的 job 下载阶段后拉迁移包：固定名资产
#   `nextrade-migrations.tgz`（发版侧由 pack-migrations.sh 按清单打包，
#   只含清单列出的迁移文件、根目录平铺），解到 $WORK_DIR/migrations/。
#   迁移包本身不进清单——完整性由 migrate-runner 逐文件对清单 sha256
#   验算锚定，包被掉换/缺文件必然在闸门处暴露。
# 补丁 3 · process_job 在 DRY_RUN 分支之后、swapping 之前插入：
#     . "$SCRIPT_DIR/updater-p3.sh"   # 或在脚本头部 source
#     hu3_migrating_phase "$JOB_FILE" "$WORK_DIR" "$BACKUP_DIR" || {
#       finish_job; return 1; }       # 终态已由阶段库写好
#   （静态 job 在阶段库内直接跳过返回 0。注意主脚本现行在 swapping 前
#   连写 backing_up/swapping 两条状态，插入点在其之前，状态序为
#   backing_up → migrating → swapping，如实反映实际顺序。）
# 补丁 4 · swapping 循环前先行后端，并把 backend 登记进 SWAPPED：
#     if [[ " $SURFACES " == *" backend "* ]]; then
#       export HU_BACKEND_BIN="${BACKEND_BIN:-/www/wwwroot/rust/nextrade}"
#       export HU_BACKEND_UNIT="${BACKEND_SERVICE:-nextrade}"
#       hu3_swap_backend_phase "$JOB_FILE" "$WORK_DIR"; brc=$?
#       case $brc in
#         0) SWAPPED+=(backend) ;;
#         4) write_status "rolled_back" "backend" "后端健康检查失败，已自动回滚旧二进制（迁移不回滚）"; finish_job; return 1 ;;
#         5) write_status "rollback_failed" "backend" "后端回滚失败，需人工介入"; finish_job; return 1 ;;
#         *) fail "后端换版失败（rc=$brc）" ;;
#       esac
#     fi
#   逐面循环里对 backend 跳过（已处理）。updater.conf 相应新增可选项
#   BACKEND_BIN / BACKEND_SERVICE（缺省即代码内默认，与现行部署一致）。
# 补丁 5 · rollback_all 加 backend 分支（静态面在后端之后失败时，
#   后端必须一并退回；迁移不退）：
#     backend)
#       . "$SCRIPT_DIR/backend-swap.sh"
#       _aside=$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1])).get("aside",""))' \
#                 "${HU_BACKEND_BIN}.hu-state.json" 2>/dev/null)
#       hu_be_rollback "$_aside" || rc=1 ;;
#
# job 文件的 P3 新增字段（后端 create_job 胶水写入）：includeMigrations
# （含 backend 时 true）、destructiveAck（破坏性执行门）。老 job 缺字段
# 按 false 处理（安全侧：不跑迁移）。
#
# 本库返回码与 migrate-runner.sh / backend-swap.sh 对齐：
# 0 成功；2 闸门；3 备份；4 已回滚/迁移失败；5 回滚失败/环境错误。

HU3_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)

# 状态写回：主 updater 已有 write_status(status stage message)（同序同
# 格式），优先委托；独立运行（演练）时用同格式自写（含 jobId 字段，
# 与主 updater 的 payload 一致）。
hu3_status() { # job_file status stage message
  if type write_status >/dev/null 2>&1; then
    write_status "$2" "$3" "$4"
    return
  fi
  local job_file=$1 status=$2 stage=$3 message=$4
  local spool status_dir job_id
  spool=$(dirname "$(dirname "$job_file")")
  status_dir="$spool/status"
  job_id=$(basename "$job_file" .json)
  mkdir -p "$status_dir" || return 1
  python3 - "$status_dir/$job_id.json" "$job_id" "$status" "$stage" "$message" <<'PYEOF'
import json, sys, time, os
path, job_id, status, stage, message = sys.argv[1:6]
doc = {"jobId": job_id, "status": status, "stage": stage,
       "message": message, "updatedAt": int(time.time())}
tmp = path + ".tmp"
with open(tmp, "w", encoding="utf-8") as fh:
    json.dump(doc, fh, ensure_ascii=False)
os.rename(tmp, path)
PYEOF
}

# 从 job 文件取一个字段（python3 解析；缺失打印空串）。
hu3_job_get() { # job_file key
  python3 - "$1" "$2" <<'PYEOF'
import json, sys
try:
    job = json.load(open(sys.argv[1], encoding="utf-8"))
except Exception:
    sys.exit(0)
v = job.get(sys.argv[2])
if isinstance(v, bool):
    print("true" if v else "false")
elif v is not None:
    print(v)
PYEOF
}

# 本 job 是否需要迁移阶段：includeMigrations=true 或 surfaces 含 backend。
hu3_needs_migration() { # job_file
  local inc
  inc=$(hu3_job_get "$1" includeMigrations)
  [ "$inc" = "true" ] && return 0
  python3 - "$1" <<'PYEOF'
import json, sys
job = json.load(open(sys.argv[1], encoding="utf-8"))
sys.exit(0 if "backend" in (job.get("surfaces") or []) else 1)
PYEOF
}

# 迁移阶段：调 migrate-runner.sh 并把退出码翻译为 job 终态（本函数负责
# 写迁移失败的终态状态；成功只写阶段进度，终态由主流程后续决定）。
# 返回 0 = 可继续 swapping；非 0 = job 已落终态，主 updater 应 finish_job
# 后退出，不得再换任何面。
hu3_migrating_phase() { # job_file work_dir backup_dir
  local job_file=$1 work_dir=$2 backup_dir=$3
  if ! hu3_needs_migration "$job_file"; then
    echo "P3 migrating phase skipped (job has no backend/migrations)"
    return 0
  fi
  local job_id spool
  job_id=$(basename "$job_file" .json)
  spool=$(dirname "$(dirname "$job_file")")
  hu3_status "$job_file" "migrating" "migrations" "正在执行数据库迁移（先备份后逐文件执行）"
  "$HU3_DIR/migrate-runner.sh" \
    --job-file "$job_file" \
    --migrations-dir "$work_dir/migrations" \
    --backup-dir "$backup_dir" \
    --result-file "$spool/status/$job_id.migrate-result.json"
  local rc=$?
  local result_file="$spool/status/$job_id.migrate-result.json"
  local failed_file="" result_msg=""
  if [ -f "$result_file" ]; then
    failed_file=$(python3 -c 'import json,sys; d=json.load(open(sys.argv[1])); print(d.get("failed") or "")' "$result_file" 2>/dev/null || echo "")
    result_msg=$(python3 -c 'import json,sys; d=json.load(open(sys.argv[1])); print(d.get("message") or "")' "$result_file" 2>/dev/null || echo "")
  fi
  case "$rc" in
    0)
      hu3_status "$job_file" "migrating" "migrations_done" "${result_msg:-迁移阶段完成}"
      return 0 ;;
    2)
      hu3_status "$job_file" "migration_failed" "gate" "${result_msg:-迁移闸门拒绝，未执行任何迁移、未换任何面}"
      return 2 ;;
    3)
      hu3_status "$job_file" "migration_failed" "backup" "${result_msg:-迁移前备份失败，未执行任何迁移、未换任何面}"
      return 3 ;;
    4)
      hu3_status "$job_file" "migration_failed" "${failed_file:-migrating}" \
        "${result_msg:-迁移执行失败，job 停在迁移阶段，未换任何面；已成功文件保持登记（迁移只进不退）}"
      return 4 ;;
    *)
      hu3_status "$job_file" "failed" "migrating" "迁移执行器异常（rc=$rc），未换任何面"
      return "$rc" ;;
  esac
}

# 后端换版阶段：从 job 清单快照取 backend 资产信息并调 backend-swap.sh。
# 只写阶段进度状态，终态由调用方（合流补丁 4）按返回码落：
# 0 成功；4 健康失败且已自动回滚；5 回滚失败；2 验包/前置失败。
hu3_swap_backend_phase() { # job_file work_dir
  local job_file=$1 work_dir=$2
  # shellcheck source=backend-swap.sh
  . "$HU3_DIR/backend-swap.sh"
  local job_id version asset_file asset_sha asset_stamp
  job_id=$(basename "$job_file" .json)
  version=$(hu3_job_get "$job_file" productVersion)
  read -r asset_file asset_sha asset_stamp <<EOF
$(python3 - "$job_file" <<'PYEOF'
import json, sys
job = json.load(open(sys.argv[1], encoding="utf-8"))
b = (job.get("manifest") or {}).get("surfaces", {}).get("backend") or {}
print(b.get("file", ""), b.get("sha256", ""), b.get("stamp", ""))
PYEOF
)
EOF
  if [ -z "$asset_file" ] || [ -z "$asset_sha" ] || [ -z "$asset_stamp" ]; then
    echo "P3 backend phase: manifest missing backend asset info" >&2
    return 2
  fi
  hu3_status "$job_file" "swapping" "backend" "正在更换后端二进制并做版本健康检查"
  hu_be_swap "$work_dir/$asset_file" "$asset_sha" "$version" "$asset_stamp" "$job_id"
  return $?
}
