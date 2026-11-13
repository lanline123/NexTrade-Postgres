#!/bin/bash
# PC 前端上线：标题契约收尾（GitHub CI build pc-build-6 / commit ad790b5）
# 包 = CI 产出 nextrade-pc.tgz（.next + public + next.config.mjs + DEPLOY_INFO.txt，无 package.json）
set -u

EXPECT_SHA=606b318d32b2493899cb8d2c4eac3baa2570860ca8874e98b1820dd9b6c340df
EXPECT_SIZE=3789463
EXPECT_BUILD=fyuS-9Iz-ocpJgG29cOGD

APP=/www/wwwroot/battery.recy.cc
SRC=/www/backup/pc_build6.tgz
SVC=nextrade-pc
TS=$(date +%Y%m%d_%H%M%S)
BK=/www/backup/pc_pre_${TS}.tgz

echo "=== 0. merge chunks ==="
cat /www/backup/pcgh_* > "$SRC" 2>/dev/null
if [ ! -s "$SRC" ]; then echo "ABORT: merged empty"; echo "##FINISHED"; exit 1; fi
GOT=$(sha256sum "$SRC" | awk '{print $1}')
SIZE=$(stat -c%s "$SRC")
echo "size=$SIZE expect=$EXPECT_SIZE"
echo "sha=$GOT"
if [ "$GOT" != "$EXPECT_SHA" ] || [ "$SIZE" != "$EXPECT_SIZE" ]; then
  echo "ABORT: bundle mismatch"; echo "##FINISHED"; exit 1; fi

echo "=== 1. pre-state ==="
echo "old_build_id=$(cat $APP/.next/BUILD_ID 2>/dev/null || echo MISSING)"
echo "svc_active=$(systemctl is-active $SVC)"
echo "disk_avail=$(df -h /www | tail -1 | awk '{print $4}')"

echo "=== 2. backup ==="
tar -czf "$BK" -C "$APP" .next public next.config.mjs DEPLOY_INFO.txt 2>/dev/null
BKS=$(stat -c%s "$BK" 2>/dev/null || echo 0)
echo "backup=$BK size=$BKS"
if [ "$BKS" -lt 1000000 ]; then echo "ABORT: backup too small"; echo "##FINISHED"; exit 1; fi

echo "=== 3. switch ==="
systemctl stop $SVC
rm -rf "$APP/.next"
tar -xzf "$SRC" -C "$APP"
echo "extract_rc=$?"
NEW_BUILD=$(cat $APP/.next/BUILD_ID 2>/dev/null || echo MISSING)
echo "new_build_id=$NEW_BUILD expect=$EXPECT_BUILD"
echo "deploy_info=$(tr '\n' ' ' < $APP/DEPLOY_INFO.txt 2>/dev/null)"
systemctl start $SVC
echo "start_rc=$?"

echo "=== 4. health ==="
code=000
for i in $(seq 1 45); do
  code=$(curl -s -o /dev/null -w '%{http_code}' --max-time 3 http://127.0.0.1:3005/)
  [ "$code" = "200" ] && break
  sleep 1
done
echo "health_3005=$code service=$(systemctl is-active $SVC)"
if [ "$code" != "200" ] || [ "$NEW_BUILD" != "$EXPECT_BUILD" ]; then
  echo "HEALTH/BUILD FAIL -> ROLLBACK"
  systemctl stop $SVC
  rm -rf "$APP/.next"
  tar -xzf "$BK" -C "$APP"
  systemctl start $SVC
  sleep 5
  echo "rollback_build_id=$(cat $APP/.next/BUILD_ID 2>/dev/null || echo MISSING)"
  echo "rollback_health=$(curl -s -o /dev/null -w '%{http_code}' --max-time 6 http://127.0.0.1:3005/)"
  echo "##FINISHED"
  exit 1
fi

echo "=== 5. verify ==="
HOMEHTML=/www/backup/pc_ci6_home.html
curl -s --max-time 30 http://127.0.0.1:3005/ > "$HOMEHTML"
echo "html_bytes=$(wc -c < $HOMEHTML)"
echo "buildid_in_html=$(grep -c $EXPECT_BUILD $HOMEHTML)"
IMGNAV=$(grep -rl 'dec-pc-image-nav__content' "$APP/.next/static/chunks/" 2>/dev/null | head -1)
echo "imgnav_chunk=$IMGNAV"
if [ -n "$IMGNAV" ]; then echo "imgnav_has_shared_title=$(grep -c 'dec-module-title' "$IMGNAV")"; fi
echo "js_legacy_recommend_h3=$(grep -rl 'dec-recommend-product__title' "$APP/.next/static/chunks/" 2>/dev/null | wc -l)"
echo "product_code=$(curl -s -o /dev/null -w '%{http_code}' --max-time 15 'http://127.0.0.1:3005/product/1559')"

echo "=== 6. cleanup ==="
rm -f /www/backup/pcgh_*
echo "backup_kept=$BK"
echo "##FINISHED"
