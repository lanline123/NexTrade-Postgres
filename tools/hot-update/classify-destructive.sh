#!/usr/bin/env bash
# 破坏性 SQL 语句扫描器（admin 自助热更新 P3，我方新增）。
#
# 设计依据：~/workspace/hot-update-design/REPORT.md §4.3 / §8 红线 6——
# DROP / TRUNCATE / 无 WHERE 的 DELETE/UPDATE 为破坏性语句，热更新通道内
# 必须在确认页单列 + 单独勾选（job 的 destructiveAck）才允许执行。
#
# 定位：这是**纵深防御**，不是唯一判定。主判定是 release-manifest.json 里
# 发版时由 classify_sql.py 生成的 `destructive` 标志；本扫描器在 updater
# 执行前对 SQL 原文复扫，两者不一致（清单说安全、原文扫出破坏性）时
# migrate-runner 直接中止 job——清单不可信时宁可不更。
#
# 用法：
#   classify-destructive.sh <file.sql> [more.sql ...]
#   source classify-destructive.sh   # 提供函数 hu_classify_destructive <file>
#
# 退出码：0 = 全部文件未扫出破坏性；3 = 至少一个文件扫出（明细打到 stdout，
# 每行一条 `DESTRUCTIVE <file> <类别> <语句摘要>`）；2 = 用法/文件错误。
#
# 已知局限（启发式扫描，非 SQL 解析器）：先剥行注释/块注释与单引号字符串
# 再按分号切分语句判定，可挡住绝大多数误报来源；动态拼接 SQL、双引号
# 标识符内含关键字等极端形态可能漏检——所以它只作第二道闸，不替代清单
# 标志与人工确认。

# 扫描单个文件；扫出破坏性时逐条打印并返回 3，干净返回 0，读不到返回 2。
hu_classify_destructive() {
  local file="$1"
  [ -f "$file" ] || { echo "ERROR classify: file not found: $file" >&2; return 2; }
  # 规范化：剥 /* */ 块注释、-- 行注释、单引号字符串字面量（'' 转义粗处理），
  # 压成单行后按 ; 切分逐句判定。
  local normalized
  normalized=$(python3 - "$file" <<'PYEOF'
import re, sys
sql = open(sys.argv[1], encoding="utf-8", errors="replace").read()
sql = re.sub(r"/\*.*?\*/", " ", sql, flags=re.S)
sql = re.sub(r"--[^\n]*", " ", sql)
sql = re.sub(r"'(?:[^']|'')*'", "''", sql)
for stmt in sql.split(";"):
    s = " ".join(stmt.split())
    if s:
        print(s)
PYEOF
  ) || { echo "ERROR classify: normalize failed: $file" >&2; return 2; }

  local found=1 stmt upper
  while IFS= read -r stmt; do
    [ -n "$stmt" ] || continue
    upper=$(printf '%s' "$stmt" | tr '[:lower:]' '[:upper:]')
    case "$upper" in
      "DROP "*)
        echo "DESTRUCTIVE $file DROP ${stmt:0:80}"; found=3 ;;
      "TRUNCATE "*)
        echo "DESTRUCTIVE $file TRUNCATE ${stmt:0:80}"; found=3 ;;
      "DELETE FROM "*)
        case "$upper" in
          *" WHERE "*) : ;;
          *) echo "DESTRUCTIVE $file DELETE_NO_WHERE ${stmt:0:80}"; found=3 ;;
        esac ;;
      "UPDATE "*)
        # UPDATE ... SET ... [WHERE ...]；WHERE 出现在 SET 之后才算数，
        # 简化为整句含 WHERE 子句即放行（表名/列名含 where 的误放由清单标志兜底）。
        case "$upper" in
          *" WHERE "*) : ;;
          *) echo "DESTRUCTIVE $file UPDATE_NO_WHERE ${stmt:0:80}"; found=3 ;;
        esac ;;
    esac
  done <<EOF
$normalized
EOF
  return $((found == 3 ? 3 : 0))
}

# 被 source 时只提供函数；直接执行时作 CLI。
if [ "${BASH_SOURCE[0]}" = "$0" ]; then
  [ $# -ge 1 ] || { echo "usage: $0 <file.sql> [...]" >&2; exit 2; }
  rc=0
  for f in "$@"; do
    hu_classify_destructive "$f" || rc=$?
    [ "$rc" = "2" ] && exit 2
  done
  exit "$rc"
fi
