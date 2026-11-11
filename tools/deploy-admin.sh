#!/bin/bash
# Admin 静态站增量部署脚本（服务器侧执行）
#
# 用途：把 GitHub CI（build-admin-frontend）产出的 nextrade-admin.tgz（dist 内容）部署到
#       /www/wwwroot/admin.recy.cc，只覆盖 index.html / assets / 404.html 等静态产物，
#       保留 pos/、pos-next/、admin-assets/ 等同级子目录。
#
# 用法：
#   BUNDLE=/www/backup/nextrade-admin.tgz EXPECT_SHA=<sha256> bash /www/backup/deploy-admin.sh
#
# 说明：通常由宝塔「计划任务」触发，输出以 ##FINISHED 结尾。
#       若脚本从 Windows 上传带 CRLF，先执行: sed -i 's/\r$//' /www/backup/deploy-admin.sh
set -u

APP=${APP:-/www/wwwroot/admin.recy.cc}
BUNDLE=${BUNDLE:-/www/backup/nextrade-admin.tgz}
EXPECT_SHA=${EXPECT_SHA:-}
HOST=${HOST:-admin.recy.cc}
TS=$(date +%Y%m%d_%H%M%S)
BK=/www/backup/admin_pre_${TS}.tgz

echo "=== 0. bundle ==="
if [ ! -f "$BUNDLE" ]; then echo "ABORT: bundle missing $BUNDLE"; echo "##FINISHED"; exit 1; fi
if [ -n "$EXPECT_SHA" ]; then
  GOT=$(sha256sum "$BUNDLE" | awk '{print $1}')
  echo "sha=$GOT expect=$EXPECT_SHA"
  if [ "$GOT" != "$EXPECT_SHA" ]; then echo "ABORT: sha mismatch"; echo "##FINISHED"; exit 1; fi
fi
echo "size=$(stat -c%s "$BUNDLE")"

echo "=== 1. backup (index.html / assets / 404.html) ==="
tar -czf "$BK" -C "$APP" index.html assets 404.html 2>/dev/null
echo "backup=$BK size=$(stat -c%s "$BK" 2>/dev/null || echo 0)"

echo "=== 2. extract (overwrite static, keep pos/pos-next/admin-assets) ==="
tar -xzf "$BUNDLE" -C "$APP"; echo "extract_rc=$?"

echo "=== 3. health ==="
code=$(curl -sL -o /dev/null -w '%{http_code}' --max-time 8 -H "Host: ${HOST}" http://127.0.0.1/)
echo "health_${HOST}=$code"
if [ "$code" != "200" ]; then
  echo "HEALTH FAIL -> ROLLBACK"
  tar -xzf "$BK" -C "$APP"
  echo "rollback_health=$(curl -sL -o /dev/null -w '%{http_code}' --max-time 8 -H "Host: ${HOST}" http://127.0.0.1/)"
  echo "##FINISHED"; exit 1
fi

echo "=== 4. verify ==="
echo "index_size=$(stat -c%s "$APP/index.html" 2>/dev/null || echo 0)"
echo "##FINISHED"
