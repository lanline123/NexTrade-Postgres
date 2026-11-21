#!/usr/bin/env bash
# 迁移包打包（admin 自助热更新 P3，我方新增）——发版侧工具。
#
# 把 release-manifest.json 里列出的迁移文件从仓库 rust/migrations/ 按
# 清单逐个验 sha 后打成固定名资产 `nextrade-migrations.tgz`（根目录
# 平铺），随统一产品 Release 发布。updater（合流补丁 2）按固定名下载
# 解包到 job 工作目录的 migrations/，migrate-runner 再逐文件对清单
# sha256 验算——包本身不进清单，完整性由逐文件 sha 锚定。
#
# 用法：pack-migrations.sh <release-manifest.json> <repo_root> <输出.tgz>
# 退出码：0 成功；2 清单内文件缺失或 sha 不符（发版流程应中止）。
set -u -o pipefail

[ $# -eq 3 ] || { echo "usage: $0 <manifest.json> <repo_root> <out.tgz>" >&2; exit 2; }
MANIFEST=$1 REPO=$2 OUT=$3
LIST=$(mktemp)
trap 'rm -f "$LIST"' EXIT

python3 - "$MANIFEST" > "$LIST" <<'PYEOF'
import json, sys
doc = json.load(open(sys.argv[1], encoding="utf-8"))
for mig in doc.get("migrations") or []:
    print(mig["file"], mig["sha256"])
PYEOF

TMPD=$(mktemp -d)
trap 'rm -rf "$TMPD" "$LIST"' EXIT
count=0
while read -r file sha; do
  [ -n "$file" ] || continue
  src="$REPO/rust/migrations/$file"
  [ -f "$src" ] || { echo "ERROR manifest migration missing in repo: $file" >&2; exit 2; }
  actual=$(sha256sum "$src" | awk '{print $1}')
  [ "$actual" = "$sha" ] || { echo "ERROR sha mismatch for $file: repo=$actual manifest=$sha" >&2; exit 2; }
  cp "$src" "$TMPD/$file"
  count=$((count+1))
done < "$LIST"

tar -czf "$OUT" -C "$TMPD" .
echo "packed $count migration file(s) -> $OUT"
