#!/bin/bash
# nextrade-updater.sh — admin 自助热更新的服务器侧独立执行器（我方新增，P2）。
#
# 设计见 ~/workspace/hot-update-design/REPORT.md §5：
# - 本脚本 root 所有、root 运行（systemd path unit 监视 spool/jobs 触发，
#   或 cron 兜底），**不在热更新范围内，只能人工部署升级**；
# - Web 进程只写 job 文件（spool/jobs/<id>.json，后端 SystemUpdateService
#   生成），本脚本消费并把状态写回 spool/status/<id>.json、日志写
#   spool/logs/<id>.log，后端读回同步落库供 admin 页面展示；
# - 状态机：queued → downloading → verifying → backing_up → swapping →
#   health_checking → done | failed → rolling_back → rolled_back /
#   rollback_failed。全局 flock 单 job 串行；
# - 本期（P2）只处理静态三面 admin/pc/h5：job 含 backend 面、或
#   schemaReady 不为 true，一律拒绝执行（后端与迁移执行在 P3）；
# - 每个资产落地必验 sha256（对 job 内钉死的 manifest 快照），不符即中止、
#   不解包；健康检查一律验内容（入口名 / BUILD_ID / Content-Type），不信裸 200；
# - 换面纪律复用现行部署口径：admin 整目录 mv 换名 + 权限归一（www 755/644）；
#   PC 只增量替换 .next（常驻 node_modules 不动）+ systemctl restart；
#   H5 只 overlay 覆盖、严禁清空（脚本内断言 static 文件数只增不减）。
#
# 配置：同目录 updater.conf（root-only，见 updater.conf.example）。
# 干跑：NEXTRADE_UPDATER_DRY_RUN=1 时只做下载+校验+包内校验，不换面。
#
# 用法：nextrade-updater.sh [job文件路径]   # 省略 = 按文件名序处理队列中全部 job

set -u
umask 022

CONF="${NEXTRADE_UPDATER_CONF:-/www/wwwroot/rust/updater/updater.conf}"
[ -f "$CONF" ] || { echo "updater.conf 不存在: $CONF" >&2; exit 2; }
# shellcheck disable=SC1090
. "$CONF"

: "${SPOOL_DIR:?updater.conf 缺 SPOOL_DIR}"
: "${TOKEN_FILE:?updater.conf 缺 TOKEN_FILE}"
BACKUP_DIR="${BACKUP_DIR:-/www/backup/hot-update}"
PC_SERVICE="${PC_SERVICE:-nextrade-pc}"
ADMIN_URL="${ADMIN_URL:-https://admin.recy.cc}"
PC_LOCAL_URL="${PC_LOCAL_URL:-http://127.0.0.1:3005}"
H5_URL="${H5_URL:-https://battery.recy.cc/mobile}"
DRY_RUN="${NEXTRADE_UPDATER_DRY_RUN:-0}"

JOBS_DIR="$SPOOL_DIR/jobs"
STATUS_DIR="$SPOOL_DIR/status"
LOGS_DIR="$SPOOL_DIR/logs"
DONE_DIR="$SPOOL_DIR/done"
WORK_ROOT="$SPOOL_DIR/work"
mkdir -p "$JOBS_DIR" "$STATUS_DIR" "$LOGS_DIR" "$DONE_DIR" "$WORK_ROOT" "$BACKUP_DIR"

exec 9>"$SPOOL_DIR/updater.lock"
flock -n 9 || { echo "updater 已在运行，本次触发跳过"; exit 0; }

JOB_ID=""
LOG_FILE=""
WORK_DIR=""
SWAPPED=()   # 已换面的面（回滚清单）
declare -A BACKUP_OF=()

log() {
  local line="[$(date '+%F %T')] $*"
  echo "$line"
  [ -n "$LOG_FILE" ] && echo "$line" >> "$LOG_FILE"
}

write_status() {
  # $1=status $2=stage $3=message
  [ -n "$JOB_ID" ] || return 0
  python3 - "$STATUS_DIR/$JOB_ID.json" "$JOB_ID" "$1" "$2" "$3" <<'PYEOF'
import json, sys, time
path, job_id, status, stage, message = sys.argv[1:6]
payload = {"jobId": job_id, "status": status, "stage": stage,
           "message": message, "updatedAt": int(time.time())}
tmp = path + ".tmp"
with open(tmp, "w", encoding="utf-8") as fh:
    json.dump(payload, fh, ensure_ascii=False)
import os
os.replace(tmp, path)
PYEOF
}

fail() {
  # $1=message；已换面则回滚，否则直接 failed
  local msg="$1"
  log "失败: $msg"
  if [ "${#SWAPPED[@]}" -gt 0 ]; then
    write_status "rolling_back" "rollback" "$msg"
    if rollback_all; then
      write_status "rolled_back" "rollback" "已自动回滚: $msg"
    else
      write_status "rollback_failed" "rollback" "自动回滚失败，需人工介入: $msg"
    fi
  else
    write_status "failed" "failed" "$msg"
  fi
  finish_job
  exit 1
}

finish_job() {
  [ -n "${JOB_FILE:-}" ] && mv -f "$JOB_FILE" "$DONE_DIR/" 2>/dev/null || true
  [ -n "$WORK_DIR" ] && rm -rf "$WORK_DIR"
}

# ————————————————————————————————————————————————————————————
# job 解析与校验（python3 输出 KEY=VALUE 到 $WORK_DIR/job.env）
# ————————————————————————————————————————————————————————————
parse_job() {
  python3 - "$JOB_FILE" "$WORK_DIR/job.env" <<'PYEOF'
import json, sys

job_path, env_path = sys.argv[1], sys.argv[2]
with open(job_path, encoding="utf-8") as fh:
    job = json.load(fh)

def die(msg):
    print(f"JOB_INVALID: {msg}", file=sys.stderr)
    sys.exit(3)

job_id = job.get("jobId") or die("缺 jobId")
surfaces = job.get("surfaces") or die("缺 surfaces")
if not isinstance(surfaces, list) or not surfaces:
    die("surfaces 必须是非空数组")
allowed = {"admin", "pc", "h5"}
for s in surfaces:
    if s == "backend":
        die("job 含 backend 面：P2 updater 拒绝执行（后端更新在 P3）")
    if s not in allowed:
        die(f"未知面: {s}")
if job.get("schemaReady") is not True:
    die("schemaReady 不为 true：前置迁移未就绪，拒绝执行")
manifest = job.get("manifest") or die("缺 manifest 快照")
msurfaces = manifest.get("surfaces") or {}
lines = [
    f"JOB_ID='{job_id}'",
    f"PRODUCT_VERSION='{manifest.get('productVersion', '')}'",
    f"RELEASE_TAG='{job.get('releaseTag', '')}'",
    f"REPO='{job.get('repo', '')}'",
    "SURFACES='" + " ".join(surfaces) + "'",
]
for s in surfaces:
    asset = msurfaces.get(s) or die(f"manifest 缺 {s} 面产物")
    fname, sha, stamp = asset.get("file", ""), asset.get("sha256", ""), asset.get("stamp", "")
    if not fname or not sha:
        die(f"{s} 面产物缺 file/sha256")
    lines.append(f"ASSET_FILE_{s.upper()}='{fname}'")
    lines.append(f"ASSET_SHA_{s.upper()}='{sha}'")
    lines.append(f"ASSET_STAMP_{s.upper()}='{stamp}'")
with open(env_path, "w", encoding="utf-8") as fh:
    fh.write("\n".join(lines) + "\n")
print(job_id)
PYEOF
}

# ————————————————————————————————————————————————————————————
# 下载（API 定位资产 → 302 两段式，令牌不随重定向外泄）
# ————————————————————————————————————————————————————————————
download_asset() {
  # $1=surface $2=asset_file $3=dest
  local surface="$1" file="$2" dest="$3"
  local token api loc
  token="$(tr -d '[:space:]' < "$TOKEN_FILE")"
  [ -n "$token" ] || fail "令牌文件为空: $TOKEN_FILE"
  api="https://api.github.com/repos/$REPO"
  local release_json="$WORK_DIR/release-$surface.json"
  curl -fsS -H "Authorization: Bearer $token" -H "Accept: application/vnd.github+json" \
       -H "User-Agent: nextrade-updater" \
       "$api/releases/tags/$RELEASE_TAG" -o "$release_json" \
    || fail "获取 Release $RELEASE_TAG 失败"
  local asset_id
  asset_id="$(python3 - "$release_json" "$file" <<'PYEOF'
import json, sys
with open(sys.argv[1], encoding="utf-8") as fh:
    rel = json.load(fh)
for a in rel.get("assets", []):
    if a.get("name") == sys.argv[2]:
        print(a["id"]); break
PYEOF
)"
  [ -n "$asset_id" ] || fail "Release $RELEASE_TAG 中没有资产 $file"
  local headers="$WORK_DIR/headers-$surface.txt"
  curl -fsS -D "$headers" -o /dev/null \
       -H "Authorization: Bearer $token" -H "Accept: application/octet-stream" \
       -H "User-Agent: nextrade-updater" \
       "$api/releases/assets/$asset_id" || fail "下载 $file 失败（资产请求）"
  loc="$(grep -i '^location:' "$headers" | tail -1 | tr -d '\r' | awk '{print $2}')"
  if [ -n "$loc" ]; then
    curl -fsS -H "User-Agent: nextrade-updater" "$loc" -o "$dest" || fail "下载 $file 失败（重定向）"
  else
    # 无重定向时 curl 已把响应体丢弃，重下一次（极少数自托管形态）
    curl -fsSL -H "Authorization: Bearer $token" -H "Accept: application/octet-stream" \
         -H "User-Agent: nextrade-updater" "$api/releases/assets/$asset_id" -o "$dest" \
      || fail "下载 $file 失败"
  fi
  [ -s "$dest" ] || fail "下载的 $file 为空"
}

verify_sha() {
  # $1=file $2=expected
  local actual
  actual="$(sha256sum "$1" | awk '{print $1}')"
  [ "$actual" = "$2" ] || fail "sha256 不符: $1 期望 $2 实际 $actual（拒绝解包）"
  log "sha256 校验通过: $1"
}

# ————————————————————————————————————————————————————————————
# 包内校验（解包后、动 live 前的硬门）
# ————————————————————————————————————————————————————————————
validate_admin_pkg() {
  local dir="$1"
  [ -f "$dir/index.html" ] || fail "admin 包缺 index.html（硬中止，未动 live）"
  local entry stamp_var="ASSET_STAMP_ADMIN"
  entry="$(grep -o 'index-[A-Za-z0-9_-]*\.js' "$dir/index.html" | head -1)"
  [ -n "$entry" ] || fail "admin 包 index.html 找不到入口 JS"
  if [ -n "${!stamp_var}" ] && [ "${!stamp_var}" != "$entry" ] && [ ! -f "$dir/version.json" ]; then
    fail "admin 包入口 $entry 与清单版本戳 ${!stamp_var} 不一致"
  fi
  echo "$entry" > "$WORK_DIR/admin.entry"
}

validate_h5_pkg() {
  local dir="$1"
  [ -f "$dir/index.html" ] || fail "H5 包缺 index.html（硬中止，未动 live）"
  local entry stamp_var="ASSET_STAMP_H5"
  entry="$(grep -o 'app\.[a-f0-9]*\.js' "$dir/index.html" | head -1)"
  [ -n "$entry" ] || fail "H5 包 index.html 找不到入口 JS"
  if [ -n "${!stamp_var}" ] && [ "${!stamp_var}" != "$entry" ] && [ ! -f "$dir/version.json" ]; then
    fail "H5 包入口 $entry 与清单版本戳 ${!stamp_var} 不一致"
  fi
  echo "$entry" > "$WORK_DIR/h5.entry"
}

validate_pc_pkg() {
  local dir="$1"
  [ -f "$dir/.next/package.json" ] || fail "PC 包缺 .next/package.json（瘦包三件套，硬中止）"
  [ -f "$dir/.next/export-marker.json" ] || fail "PC 包缺 .next/export-marker.json（硬中止）"
  [ -f "$dir/.next/images-manifest.json" ] || fail "PC 包缺 .next/images-manifest.json（硬中止）"
  [ -f "$dir/.next/BUILD_ID" ] || fail "PC 包缺 .next/BUILD_ID（硬中止）"
  [ -f "$dir/DEPLOY_INFO.txt" ] || fail "PC 包缺 DEPLOY_INFO.txt（硬中止）"
  local bid stamp_var="ASSET_STAMP_PC"
  bid="$(tr -d '[:space:]' < "$dir/.next/BUILD_ID")"
  if [ -n "${!stamp_var}" ] && [ "${!stamp_var}" != "$bid" ]; then
    fail "PC 包 BUILD_ID $bid 与清单版本戳 ${!stamp_var} 不一致"
  fi
  echo "$bid" > "$WORK_DIR/pc.buildid"
}

# ————————————————————————————————————————————————————————————
# 换面与回滚
# ————————————————————————————————————————————————————————————
TS="$(date +%Y%m%d%H%M%S)"

swap_admin() {
  local src="$WORK_DIR/pkg-admin"
  local backup="$BACKUP_DIR/admin_pre_hu_${JOB_ID:0:8}_$TS"
  if [ -d "$ADMIN_ROOT" ]; then
    mv "$ADMIN_ROOT" "$backup" || fail "admin 备份失败"
    BACKUP_OF[admin]="$backup"
  fi
  mv "$src" "$ADMIN_ROOT" || fail "admin 换面失败"
  # 权限归一（v54 全站 403 教训）：www 可读、目录 755 文件 644
  chown -R www:www "$ADMIN_ROOT" 2>/dev/null || log "警告：chown www 失败，请人工检查 admin 目录权限"
  find "$ADMIN_ROOT" -type d -exec chmod 755 {} + 
  find "$ADMIN_ROOT" -type f -exec chmod 644 {} +
  SWAPPED+=(admin)
  log "admin 已换面（备份: $backup）"
}

swap_pc() {
  local src="$WORK_DIR/pkg-pc"
  local backup="$BACKUP_DIR/pc_next_pre_hu_${JOB_ID:0:8}_$TS"
  mkdir -p "$backup"
  if [ -d "$PC_ROOT/.next" ]; then
    mv "$PC_ROOT/.next" "$backup/next" || fail "PC .next 备份失败"
  fi
  [ -f "$PC_ROOT/DEPLOY_INFO.txt" ] && cp -a "$PC_ROOT/DEPLOY_INFO.txt" "$backup/"
  [ -f "$PC_ROOT/next.config.mjs" ] && cp -a "$PC_ROOT/next.config.mjs" "$backup/"
  BACKUP_OF[pc]="$backup"
  cp -a "$src/.next" "$PC_ROOT/.next" || fail "PC .next 换面失败"
  cp -a "$src/DEPLOY_INFO.txt" "$PC_ROOT/DEPLOY_INFO.txt" || fail "PC DEPLOY_INFO 换面失败"
  [ -f "$src/next.config.mjs" ] && cp -a "$src/next.config.mjs" "$PC_ROOT/next.config.mjs"
  if [ -d "$src/public" ]; then cp -a "$src/public/." "$PC_ROOT/public/" 2>/dev/null || true; fi
  chown -R www:www "$PC_ROOT/.next" 2>/dev/null || true
  SWAPPED+=(pc)
  systemctl restart "$PC_SERVICE" || fail "PC 服务重启失败"
  log "PC 已换面并重启（备份: $backup）"
}

swap_h5() {
  local src="$WORK_DIR/pkg-h5"
  local backup="$BACKUP_DIR/mobile_pre_hu_${JOB_ID:0:8}_$TS"
  local before=0 after=0
  [ -d "$H5_ROOT/static" ] && before="$(find "$H5_ROOT/static" -type f | wc -l)"
  cp -a "$H5_ROOT" "$backup" || fail "H5 备份失败"
  BACKUP_OF[h5]="$backup"
  # overlay 覆盖：只 cp 不删（严禁 clear-first）
  cp -a "$src/." "$H5_ROOT/" || fail "H5 overlay 覆盖失败"
  [ -d "$H5_ROOT/static" ] && after="$(find "$H5_ROOT/static" -type f | wc -l)"
  if [ "$after" -lt "$before" ]; then
    fail "H5 static 文件数减少（$before → $after），违反 overlay 纪律"
  fi
  chown -R www:www "$H5_ROOT" 2>/dev/null || true
  SWAPPED+=(h5)
  log "H5 已 overlay 换面（备份: $backup，static $before → $after）"
}

rollback_all() {
  local rc=0 s
  for s in "${SWAPPED[@]}"; do
    case "$s" in
      admin)
        rm -rf "$ADMIN_ROOT"
        if [ -n "${BACKUP_OF[admin]:-}" ]; then
          mv "${BACKUP_OF[admin]}" "$ADMIN_ROOT" || rc=1
        fi
        ;;
      pc)
        rm -rf "$PC_ROOT/.next"
        if [ -d "${BACKUP_OF[pc]}/next" ]; then
          mv "${BACKUP_OF[pc]}/next" "$PC_ROOT/.next" || rc=1
        fi
        [ -f "${BACKUP_OF[pc]}/DEPLOY_INFO.txt" ] && cp -a "${BACKUP_OF[pc]}/DEPLOY_INFO.txt" "$PC_ROOT/"
        [ -f "${BACKUP_OF[pc]}/next.config.mjs" ] && cp -a "${BACKUP_OF[pc]}/next.config.mjs" "$PC_ROOT/"
        systemctl restart "$PC_SERVICE" || rc=1
        ;;
      h5)
        find "$H5_ROOT" -mindepth 1 -delete
        cp -a "${BACKUP_OF[h5]}/." "$H5_ROOT/" || rc=1
        ;;
    esac
    log "已回滚: $s"
  done
  return $rc
}

# ————————————————————————————————————————————————————————————
# 健康检查（验内容不验状态码）
# ————————————————————————————————————————————————————————————
health_admin() {
  local entry expect_html i
  entry="$(cat "$WORK_DIR/admin.entry")"
  for i in 1 2 3 4 5; do
    expect_html="$(curl -fsS "$ADMIN_URL/" 2>/dev/null)" || expect_html=""
    case "$expect_html" in
      *"$entry"*) log "admin 健康检查通过（入口 $entry）"; return 0 ;;
    esac
    sleep 2
  done
  fail "admin 健康检查失败：首页未出现新入口 $entry"
}

health_pc() {
  local bid i html
  bid="$(cat "$WORK_DIR/pc.buildid")"
  for i in $(seq 1 15); do
    html="$(curl -fsS "$PC_LOCAL_URL/" 2>/dev/null)" || html=""
    if [ -n "$html" ]; then
      local live_bid
      live_bid="$(tr -d '[:space:]' < "$PC_ROOT/.next/BUILD_ID" 2>/dev/null)"
      [ "$live_bid" = "$bid" ] || fail "PC 健康检查失败：live BUILD_ID $live_bid ≠ $bid"
      log "PC 健康检查通过（BUILD_ID $bid）"
      return 0
    fi
    sleep 2
  done
  fail "PC 健康检查失败：$PC_LOCAL_URL 无响应"
}

health_h5() {
  local entry html i
  entry="$(cat "$WORK_DIR/h5.entry")"
  for i in 1 2 3 4 5; do
    html="$(curl -fsS "$H5_URL/" 2>/dev/null)" || html=""
    case "$html" in
      *"$entry"*) break ;;
      *) sleep 2 ;;
    esac
  done
  case "$html" in
    *"$entry"*) : ;;
    *) fail "H5 健康检查失败：首页未出现新入口 $entry" ;;
  esac
  # 抽一个包内静态资源验 Content-Type（SPA fallback 对缺失路径回 text/html，不能当成功）
  local sample ctype
  sample="$(cd "$WORK_DIR/pkg-h5" 2>/dev/null && find static -type f \( -name '*.png' -o -name '*.js' -o -name '*.css' \) 2>/dev/null | head -1)"
  if [ -n "$sample" ]; then
    ctype="$(curl -fsSI "$H5_URL/$sample" 2>/dev/null | grep -i '^content-type:' | tail -1)"
    case "$ctype" in
      *text/html*) fail "H5 健康检查失败：静态资源 $sample 返回 text/html（疑似 SPA fallback）" ;;
      *) log "H5 静态抽查通过: $sample ($ctype)" ;;
    esac
  fi
  log "H5 健康检查通过（入口 $entry）"
}

# ————————————————————————————————————————————————————————————
# 主流程
# ————————————————————————————————————————————————————————————
process_job() {
  JOB_FILE="$1"
  WORK_DIR="$WORK_ROOT/$(basename "$JOB_FILE" .json)"
  rm -rf "$WORK_DIR"; mkdir -p "$WORK_DIR"
  SWAPPED=()
  BACKUP_OF=()

  if ! JOB_ID="$(parse_job 2>"$WORK_DIR/parse.err")"; then
    # job 本身非法：尽力从文件名取 id 写状态，避免页面永远 queued
    JOB_ID="$(basename "$JOB_FILE" .json)"
    LOG_FILE="$LOGS_DIR/$JOB_ID.log"
    log "job 校验失败: $(cat "$WORK_DIR/parse.err")"
    write_status "failed" "validate" "job 描述非法: $(cat "$WORK_DIR/parse.err")"
    mv -f "$JOB_FILE" "$DONE_DIR/" 2>/dev/null || true
    rm -rf "$WORK_DIR"
    return 1
  fi
  LOG_FILE="$LOGS_DIR/$JOB_ID.log"
  # shellcheck disable=SC1090
  . "$WORK_DIR/job.env"
  log "开始处理 job $JOB_ID（目标版本 $PRODUCT_VERSION，面: $SURFACES）"

  # downloading + verifying
  write_status "downloading" "download" "下载 Release $RELEASE_TAG 资产"
  local s
  for s in $SURFACES; do
    local file_var="ASSET_FILE_${s^^}" sha_var="ASSET_SHA_${s^^}"
    download_asset "$s" "${!file_var}" "$WORK_DIR/${!file_var}"
    write_status "verifying" "verify" "校验 ${!file_var} 的 sha256"
    verify_sha "$WORK_DIR/${!file_var}" "${!sha_var}"
    mkdir -p "$WORK_DIR/pkg-$s"
    tar -xzf "$WORK_DIR/${!file_var}" -C "$WORK_DIR/pkg-$s" || fail "解包 ${!file_var} 失败"
  done

  # 包内硬校验（动 live 之前）
  for s in $SURFACES; do
    case "$s" in
      admin) validate_admin_pkg "$WORK_DIR/pkg-admin" ;;
      pc) validate_pc_pkg "$WORK_DIR/pkg-pc" ;;
      h5) validate_h5_pkg "$WORK_DIR/pkg-h5" ;;
    esac
  done

  if [ "$DRY_RUN" = "1" ]; then
    write_status "done" "dry_run" "干跑完成：下载、sha 校验与包内校验全部通过，未换面"
    log "干跑完成（job $JOB_ID）"
    finish_job
    return 0
  fi

  # swapping（含各面备份）
  write_status "backing_up" "backup" "备份现行版本"
  write_status "swapping" "swap" "逐面换版"
  for s in $SURFACES; do
    case "$s" in
      admin) swap_admin ;;
      pc) swap_pc ;;
      h5) swap_h5 ;;
    esac
  done

  # health_checking
  write_status "health_checking" "health" "健康检查（验内容）"
  for s in $SURFACES; do
    case "$s" in
      admin) health_admin ;;
      pc) health_pc ;;
      h5) health_h5 ;;
    esac
  done

  write_status "done" "done" "更新完成：v$PRODUCT_VERSION（面: $SURFACES）"
  log "job $JOB_ID 完成"
  finish_job
  return 0
}

if [ "${1:-}" != "" ]; then
  process_job "$1"
else
  shopt -s nullglob
  jobs=("$JOBS_DIR"/*.json)
  if [ "${#jobs[@]}" -eq 0 ]; then
    echo "无待处理 job"
    exit 0
  fi
  for jf in "${jobs[@]}"; do
    ( process_job "$jf" ) || true
  done
fi
