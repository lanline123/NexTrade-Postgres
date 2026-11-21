#!/usr/bin/env bash
# 后端二进制换版模块（admin 自助热更新 P3，我方新增）。
#
# 设计依据：~/workspace/hot-update-design/REPORT.md §5.1/§5.2——后端换版
# 只能由独立 updater 执行（Web 进程不自改），本模块实现状态机 swapping
# 阶段的后端部分：验包 → 旧件 mv-aside 留回滚 → 装新件 → systemctl
# restart → 版本端点健康检查 → 失败自动回滚旧件并复探活。
#
# 两种用法：
#   1) 被 updater-p3.sh source：提供 hu_be_swap / hu_be_rollback /
#      hu_be_health_version / hu_be_health_liveness 函数；
#   2) 独立执行（人工/演练）：backend-swap.sh --asset <新二进制> \
#        --expect-sha <sha256> --expect-version <产品版本> \
#        --expect-git-sha <清单 backend.stamp> --job-id <id>
#
# 环境变量（均可覆盖，演练靠覆盖指向沙箱）：
#   HU_BACKEND_BIN        二进制路径（默认 /www/wwwroot/rust/nextrade）
#   HU_BACKEND_UNIT       systemd 单元名（默认 nextrade）
#   HU_BACKEND_HEALTH_URLS 版本端点候选（空格分隔，任一命中即算通；
#                         默认本机 18080/18181 的 /api/version——P1 自报端点）
#   HU_BACKEND_LEGACY_URLS 回滚后旧版探活候选（旧二进制可能没有 /version
#                         端点，用存量只读接口探活）
#   HU_HEALTH_TIMEOUT_SECS 健康检查总超时（默认 90）
#   HU_HEALTH_INTERVAL_SECS 轮询间隔（默认 3）
#   HU_SYSTEMCTL / HU_CURL 二进制覆盖（演练注桩用）
#
# 健康口径（设计 §5.2：验内容不验状态码）：新件必须在超时内让某个版本
# 端点返回 productVersion 与 gitSha **同时**等于期望值才算成功；仅进程
# 活着/端口通不算。回滚验证分两级：版本端点若响应且 gitSha 等于旧件
# 记录的 stamp 最佳；旧件无版本端点时降级为「单元 active + 任一存量
# 只读接口 HTTP 200 且有响应体」，并在输出中如实标注降级。
#
# 退出/返回码：0 成功；4 健康检查失败且已自动回滚成功（旧版在服务）；
# 5 回滚失败（人工介入）；2 验包/前置失败（未动现行二进制）。
#
# 状态文件：换版成功/回滚信息写 <HU_BACKEND_BIN>.hu-state.json
# {jobId, aside, prevSha, newSha, productVersion, swappedAt}，供后续
# job 或人工按图回滚；回滚件命名 <bin>_pre_hotupdate_<job 前 8 位>，
# 沿用现行 nextrade_pre_<轮次> 纪律、至少保留上一版（红线 12）。

HU_BACKEND_BIN=${HU_BACKEND_BIN:-/www/wwwroot/rust/nextrade}
HU_BACKEND_UNIT=${HU_BACKEND_UNIT:-nextrade}
HU_BACKEND_HEALTH_URLS=${HU_BACKEND_HEALTH_URLS:-"http://127.0.0.1:18080/api/version http://127.0.0.1:18181/api/version"}
HU_BACKEND_LEGACY_URLS=${HU_BACKEND_LEGACY_URLS:-"http://127.0.0.1:18181/api/common/config/themeSettings http://127.0.0.1:18080/api/home/index"}
HU_HEALTH_TIMEOUT_SECS=${HU_HEALTH_TIMEOUT_SECS:-90}
HU_HEALTH_INTERVAL_SECS=${HU_HEALTH_INTERVAL_SECS:-3}
HU_SYSTEMCTL=${HU_SYSTEMCTL:-systemctl}
HU_CURL=${HU_CURL:-curl}

_hu_be_sha() { sha256sum "$1" 2>/dev/null | awk '{print $1}'; }

_hu_be_version_probe() { # url -> 打印 "productVersion gitSha"，失败返回 1
  local body
  body=$("$HU_CURL" -fsS -m 5 "$1" 2>/dev/null) || return 1
  [ -n "$body" ] || return 1
  printf '%s' "$body" | python3 -c '
import json, sys
try:
    doc = json.load(sys.stdin)
except Exception:
    sys.exit(1)
def find(obj):
    if isinstance(obj, dict):
        if "gitSha" in obj and "productVersion" in obj:
            return obj
        for v in obj.values():
            r = find(v)
            if r is not None:
                return r
    return None
hit = find(doc)
if hit is None:
    sys.exit(1)
print(hit["productVersion"], hit["gitSha"])
' 2>/dev/null
}

# 版本健康检查：超时内任一候选端点返回期望版本+sha 即成功。
hu_be_health_version() { # expect_version expect_git_sha
  local deadline=$(( $(date +%s) + HU_HEALTH_TIMEOUT_SECS )) url got
  while [ "$(date +%s)" -lt "$deadline" ]; do
    if "$HU_SYSTEMCTL" is-active --quiet "$HU_BACKEND_UNIT" 2>/dev/null; then
      for url in $HU_BACKEND_HEALTH_URLS; do
        if got=$(_hu_be_version_probe "$url"); then
          if [ "$got" = "$1 $2" ]; then
            echo "HEALTH version ok ($url): $got"
            return 0
          fi
        fi
      done
    fi
    sleep "$HU_HEALTH_INTERVAL_SECS"
  done
  echo "HEALTH version FAILED: expect '$1 $2' within ${HU_HEALTH_TIMEOUT_SECS}s" >&2
  return 1
}

# 探活检查（回滚验证用）：单元 active + 任一存量接口 200 有体。
hu_be_health_liveness() {
  local deadline=$(( $(date +%s) + HU_HEALTH_TIMEOUT_SECS )) url code
  while [ "$(date +%s)" -lt "$deadline" ]; do
    if "$HU_SYSTEMCTL" is-active --quiet "$HU_BACKEND_UNIT" 2>/dev/null; then
      for url in $HU_BACKEND_LEGACY_URLS; do
        code=$("$HU_CURL" -s -o /dev/null -m 5 -w '%{http_code}' "$url" 2>/dev/null || echo 000)
        if [ "$code" = "200" ]; then
          echo "HEALTH liveness ok ($url)"
          return 0
        fi
      done
    fi
    sleep "$HU_HEALTH_INTERVAL_SECS"
  done
  echo "HEALTH liveness FAILED within ${HU_HEALTH_TIMEOUT_SECS}s" >&2
  return 1
}

_hu_be_write_state() { # job_id aside prev_sha new_sha version
  python3 - "$HU_BACKEND_BIN.hu-state.json" "$1" "$2" "$3" "$4" "$5" <<'PYEOF'
import json, sys, time
path, job_id, aside, prev, new, version = sys.argv[1:7]
doc = {"jobId": job_id, "aside": aside, "prevSha": prev, "newSha": new,
       "productVersion": version, "swappedAt": int(time.time())}
with open(path, "w", encoding="utf-8") as fh:
    json.dump(doc, fh, ensure_ascii=False, indent=2)
PYEOF
}

# 回滚：把 aside 件还原为现行二进制并重启、探活。
hu_be_rollback() { # aside_path [prev_git_sha]
  local aside=$1 prev_git=${2:-}
  [ -n "$aside" ] && [ -f "$aside" ] || { echo "ROLLBACK no aside file: $aside" >&2; return 5; }
  echo "ROLLBACK restoring $aside -> $HU_BACKEND_BIN"
  cp -a "$aside" "$HU_BACKEND_BIN.hu-rollback.tmp" || return 5
  chmod 755 "$HU_BACKEND_BIN.hu-rollback.tmp"
  mv "$HU_BACKEND_BIN.hu-rollback.tmp" "$HU_BACKEND_BIN" || return 5
  "$HU_SYSTEMCTL" restart "$HU_BACKEND_UNIT" || {
    echo "ROLLBACK restart command failed" >&2; return 5; }
  # 旧件若带版本端点且旧 gitSha 已知，先按 gitSha 严验（超时内轮询）；
  # 旧件无版本端点（P1 之前的二进制）则走下面的探活降级。
  if [ -n "$prev_git" ]; then
    local deadline=$(( $(date +%s) + HU_HEALTH_TIMEOUT_SECS )) url got
    while [ "$(date +%s)" -lt "$deadline" ]; do
      if "$HU_SYSTEMCTL" is-active --quiet "$HU_BACKEND_UNIT" 2>/dev/null; then
        for url in $HU_BACKEND_HEALTH_URLS; do
          if got=$(_hu_be_version_probe "$url"); then
            if [ "${got##* }" = "$prev_git" ]; then
              echo "ROLLBACK ok (version endpoint confirms previous gitSha: $got)"
              return 0
            fi
          fi
        done
      fi
      sleep "$HU_HEALTH_INTERVAL_SECS"
    done
    echo "ROLLBACK version probe did not confirm previous gitSha; falling back to liveness" >&2
  fi
  if hu_be_health_liveness; then
    local now_sha
    now_sha=$(_hu_be_sha "$HU_BACKEND_BIN")
    echo "ROLLBACK ok (liveness; installed sha=$now_sha)"
    return 0
  fi
  echo "ROLLBACK FAILED: previous binary restored but not serving" >&2
  return 5
}

# 换版主流程。成功 0；健康失败已回滚 4；回滚失败 5；前置失败 2。
hu_be_swap() { # asset expect_sha expect_version expect_git_sha job_id
  local asset=$1 expect_sha=$2 expect_version=$3 expect_git=$4 job_id=$5
  [ -f "$asset" ] || { echo "SWAP asset missing: $asset" >&2; return 2; }
  local asset_sha
  asset_sha=$(_hu_be_sha "$asset")
  [ "$asset_sha" = "$expect_sha" ] || {
    echo "SWAP sha mismatch: asset=$asset_sha expect=$expect_sha" >&2; return 2; }
  head -c 4 "$asset" | grep -q "$(printf '\177')ELF" || {
    echo "SWAP asset is not an ELF binary" >&2; return 2; }

  # 换版前先探一遍现行服务的 gitSha（旧件带 P1 版本端点时可得），供回滚
  # 后严验「确实回到了旧版本」；探不到（旧件无端点）则回滚走探活降级。
  local prev_git_stamp="" _url _got
  for _url in $HU_BACKEND_HEALTH_URLS; do
    if _got=$(_hu_be_version_probe "$_url"); then
      prev_git_stamp=${_got##* }
      break
    fi
  done
  [ -n "$prev_git_stamp" ] && echo "SWAP current serving gitSha: $prev_git_stamp"

  local prev_sha="" aside=""
  if [ -f "$HU_BACKEND_BIN" ]; then
    prev_sha=$(_hu_be_sha "$HU_BACKEND_BIN")
    aside="${HU_BACKEND_BIN}_pre_hotupdate_${job_id:0:8}"
    [ -e "$aside" ] && aside="${aside}-$(date +%s)"
    cp -a "$HU_BACKEND_BIN" "$aside" || {
      echo "SWAP cannot create rollback copy: $aside" >&2; return 2; }
    [ "$(_hu_be_sha "$aside")" = "$prev_sha" ] || {
      echo "SWAP rollback copy sha mismatch" >&2; return 2; }
    echo "SWAP rollback copy: $aside (sha=$prev_sha)"
  else
    echo "SWAP no existing binary (first install); no rollback copy"
  fi

  cp "$asset" "$HU_BACKEND_BIN.hu-new.tmp" || return 2
  chmod 755 "$HU_BACKEND_BIN.hu-new.tmp"
  mv "$HU_BACKEND_BIN.hu-new.tmp" "$HU_BACKEND_BIN" || return 2
  [ "$(_hu_be_sha "$HU_BACKEND_BIN")" = "$expect_sha" ] || {
    echo "SWAP installed sha mismatch after mv" >&2
    [ -n "$aside" ] && hu_be_rollback "$aside" "$prev_git_stamp"
    return 5
  }
  _hu_be_write_state "$job_id" "$aside" "$prev_sha" "$expect_sha" "$expect_version" || true

  "$HU_SYSTEMCTL" restart "$HU_BACKEND_UNIT" || echo "SWAP restart returned non-zero; health check decides" >&2
  if hu_be_health_version "$expect_version" "$expect_git"; then
    echo "BACKEND_SWAP OK version=$expect_version git=$expect_git aside=$aside"
    return 0
  fi
  echo "SWAP health check failed; auto-rolling back" >&2
  if [ -n "$aside" ]; then
    if hu_be_rollback "$aside" "$prev_git_stamp"; then
      echo "BACKEND_SWAP ROLLED_BACK (previous binary serving; schema NOT rolled back)"
      return 4
    fi
    echo "BACKEND_SWAP ROLLBACK_FAILED"
    return 5
  fi
  echo "BACKEND_SWAP FAILED (no rollback copy existed)"
  return 5
}

# 独立执行入口（被 source 时不触发）。
if [ "${BASH_SOURCE[0]}" = "$0" ]; then
  set -u -o pipefail
  ASSET="" EXPECT_SHA="" EXPECT_VERSION="" EXPECT_GIT="" JOB_ID="manual"
  while [ $# -gt 0 ]; do
    case "$1" in
      --asset) ASSET=$2; shift 2 ;;
      --expect-sha) EXPECT_SHA=$2; shift 2 ;;
      --expect-version) EXPECT_VERSION=$2; shift 2 ;;
      --expect-git-sha) EXPECT_GIT=$2; shift 2 ;;
      --job-id) JOB_ID=$2; shift 2 ;;
      --rollback) # 按状态文件回滚
        ASIDE=$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1])).get("aside",""))' "$HU_BACKEND_BIN.hu-state.json" 2>/dev/null || echo "")
        hu_be_rollback "$ASIDE"; exit $? ;;
      *) echo "ERROR unknown arg: $1" >&2; exit 2 ;;
    esac
  done
  [ -n "$ASSET" ] && [ -n "$EXPECT_SHA" ] && [ -n "$EXPECT_GIT" ] || {
    echo "usage: backend-swap.sh --asset F --expect-sha S --expect-version V --expect-git-sha G [--job-id J] | --rollback" >&2
    exit 2
  }
  hu_be_swap "$ASSET" "$EXPECT_SHA" "$EXPECT_VERSION" "$EXPECT_GIT" "$JOB_ID"
  exit $?
fi
