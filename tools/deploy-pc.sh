#!/bin/bash
# PC 增量部署脚本（服务器侧执行）
#
# 用途：把 GitHub CI（build-pc-frontend）产出的 nextrade-pc.tgz 部署到
#       /www/wwwroot/battery.recy.cc。包内只有 .next / public / next.config.mjs / DEPLOY_INFO.txt，
#       脚本只替换 .next 等产物，不动 node_modules / package.json / scripts。
#
# 前提：
#   1. 包已上传到 /www/backup/nextrade-pc.tgz（大文件用分片上传后 cat 合并）
#   2. 以 root 执行（systemctl 需要）
#
# 用法：
#   BUNDLE=/www/backup/nextrade-pc.tgz EXPECT_SHA=<sha256> bash /www/backup/deploy-pc.sh
#
# 说明：通常由宝塔「计划任务」触发，输出以 ##FINISHED 结尾，便于 GetFileBody 轮询。
#       若脚本从 Windows 上传带 CRLF，先执行: sed -i 's/\r$//' /www/backup/deploy-pc.sh
set -u

APP=${APP:-/www/wwwroot/battery.recy.cc}
BUNDLE=${BUNDLE:-/www/backup/nextrade-pc.tgz}
EXPECT_SHA=${EXPECT_SHA:-}
SERVICE=${SERVICE:-nextrade-pc}
PORT=${PORT:-3005}
TS=$(date +%Y%m%d_%H%M%S)
BK=/www/backup/pc_pre_${TS}.tgz

echo "=== 0. bundle ==="
if [ ! -f "$BUNDLE" ]; then echo "ABORT: bundle missing $BUNDLE"; echo "##FINISHED"; exit 1; fi
if [ -n "$EXPECT_SHA" ]; then
  GOT=$(sha256sum "$BUNDLE" | awk '{print $1}')
  echo "sha=$GOT expect=$EXPECT_SHA"
  if [ "$GOT" != "$EXPECT_SHA" ]; then echo "ABORT: sha mismatch"; echo "##FINISHED"; exit 1; fi
fi
echo "size=$(stat -c%s "$BUNDLE")"

echo "=== 1. pre-state ==="
echo "old_build_id=$(cat $APP/.next/BUILD_ID 2>/dev/null || echo MISSING)"
echo "old_deploy_info=$(tr '\n' ' ' < $APP/DEPLOY_INFO.txt 2>/dev/null)"

echo "=== 2. backup ==="
tar -czf "$BK" -C "$APP" .next public next.config.mjs DEPLOY_INFO.txt 2>/dev/null
BKS=$(stat -c%s "$BK" 2>/dev/null || echo 0)
echo "backup=$BK size=$BKS"
if [ "$BKS" -lt 100000 ]; then echo "ABORT: backup too small"; echo "##FINISHED"; exit 1; fi

echo "=== 3. switch ==="
systemctl stop "$SERVICE"; echo "stop_rc=$?"
rm -rf "$APP/.next"
tar -xzf "$BUNDLE" -C "$APP"; echo "extract_rc=$?"
NEW_BUILD=$(cat "$APP/.next/BUILD_ID" 2>/dev/null || echo MISSING)
echo "new_build_id=$NEW_BUILD"
systemctl start "$SERVICE"; echo "start_rc=$?"

echo "=== 4. health ==="
code=000
for i in $(seq 1 40); do
  code=$(curl -s -o /dev/null -w '%{http_code}' --max-time 3 "http://127.0.0.1:${PORT}/")
  [ "$code" = "200" ] && break
  sleep 1
done
echo "health_${PORT}=$code service=$(systemctl is-active "$SERVICE")"
if [ "$code" != "200" ]; then
  echo "HEALTH FAIL -> ROLLBACK"
  systemctl stop "$SERVICE"
  rm -rf "$APP/.next"
  tar -xzf "$BK" -C "$APP"
  systemctl start "$SERVICE"
  sleep 4
  echo "rollback_build_id=$(cat $APP/.next/BUILD_ID 2>/dev/null || echo MISSING)"
  echo "rollback_health=$(curl -s -o /dev/null -w '%{http_code}' --max-time 5 "http://127.0.0.1:${PORT}/")"
  echo "##FINISHED"; exit 1
fi

echo "=== 5. verify ==="
echo "deploy_info=$(tr '\n' ' ' < $APP/DEPLOY_INFO.txt 2>/dev/null)"
echo "##FINISHED"
