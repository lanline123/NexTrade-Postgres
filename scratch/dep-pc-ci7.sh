#!/bin/bash
# PC 上线：commit fb0cf15（GitHub CI pc-build-7）——标题左右边距 + 秒杀卡片区对齐预览 + 推荐商品默认值
set -u
EXPECT_SHA=b4f4a294d21a0099a91fef88ed80695c291d802e3e6d740cee540f618c924a73
EXPECT_SIZE=3787987
EXPECT_BUILD=0nYo1M62GXJOSy2lwFEPd
APP=/www/wwwroot/battery.recy.cc
SRC=/www/backup/pc_ci7.tgz
SVC=nextrade-pc
TS=$(date +%Y%m%d_%H%M%S)
BK=/www/backup/pc_pre_${TS}.tgz
echo "=== 0. merge chunks ==="
cat /www/backup/pcg2_* > "$SRC" 2>/dev/null
[ -s "$SRC" ] || { echo "ABORT: merged empty"; echo "##FINISHED"; exit 1; }
GOT=$(sha256sum "$SRC" | awk '{print $1}'); SIZE=$(stat -c%s "$SRC")
echo "size=$SIZE expect=$EXPECT_SIZE"; echo "sha=$GOT"
if [ "$GOT" != "$EXPECT_SHA" ] || [ "$SIZE" != "$EXPECT_SIZE" ]; then echo "ABORT: mismatch"; echo "##FINISHED"; exit 1; fi
echo "=== 1. pre ==="; echo "old_build_id=$(cat $APP/.next/BUILD_ID 2>/dev/null || echo MISSING)"; echo "svc=$(systemctl is-active $SVC)"
echo "=== 2. backup ==="; tar -czf "$BK" -C "$APP" .next public next.config.mjs DEPLOY_INFO.txt 2>/dev/null; BKS=$(stat -c%s "$BK" 2>/dev/null || echo 0); echo "backup=$BK size=$BKS"
[ "$BKS" -gt 1000000 ] || { echo "ABORT: backup too small"; echo "##FINISHED"; exit 1; }
echo "=== 3. switch ==="; systemctl stop $SVC; rm -rf "$APP/.next"; tar -xzf "$SRC" -C "$APP"; echo "extract_rc=$?"
NEW=$(cat $APP/.next/BUILD_ID 2>/dev/null || echo MISSING); echo "new_build_id=$NEW expect=$EXPECT_BUILD"
echo "deploy_info=$(tr '\n' ' ' < $APP/DEPLOY_INFO.txt 2>/dev/null)"
systemctl start $SVC; echo "start_rc=$?"
echo "=== 4. health ==="
code=000; for i in $(seq 1 45); do code=$(curl -s -o /dev/null -w '%{http_code}' --max-time 3 http://127.0.0.1:3005/); [ "$code" = "200" ] && break; sleep 1; done
echo "health_3005=$code service=$(systemctl is-active $SVC)"
if [ "$code" != "200" ] || [ "$NEW" != "$EXPECT_BUILD" ]; then
  echo "FAIL -> ROLLBACK"; systemctl stop $SVC; rm -rf "$APP/.next"; tar -xzf "$BK" -C "$APP"; systemctl start $SVC; sleep 5
  echo "rollback_build_id=$(cat $APP/.next/BUILD_ID 2>/dev/null || echo MISSING)"; echo "rollback_health=$(curl -s -o /dev/null -w '%{http_code}' --max-time 6 http://127.0.0.1:3005/)"; echo "##FINISHED"; exit 1
fi
echo "=== 5. verify ==="
H=/www/backup/pc_ci7_home.html; curl -s --max-time 30 http://127.0.0.1:3005/ > "$H"
echo "html_bytes=$(wc -c < $H)"; echo "buildid_in_html=$(grep -c $EXPECT_BUILD $H)"
echo "js_titlepad=$(grep -rl 'titlePaddingLeft' "$APP/.next/static/chunks/" 2>/dev/null | wc -l)"
echo "js_style_preview=$(grep -rl 'ad-style__preview\|styleExplicit' "$APP/.next/static/chunks/" 2>/dev/null | wc -l)"
echo "product_code=$(curl -s -o /dev/null -w '%{http_code}' --max-time 15 http://127.0.0.1:3005/product/1559)"
echo "=== 6. cleanup ==="; rm -f /www/backup/pcg2_*; echo "backup_kept=$BK"; echo "##FINISHED"
