#!/bin/bash
# Admin 上线：admin-build-6（推荐商品默认背景 transparent）
set -u
EXPECT_SHA=8a3f9f1d23f9707f64f28e09e8cced0dee05326b7238331217a0a01f990a9427
EXPECT_SIZE=4772363
EXPECT_ENTRY=index-DKPth07v.js
EXPECT_CSS=index-CnNv8buu.css
APP=/www/wwwroot/admin.recy.cc
SRC=/www/backup/admin_ci6.tgz
TS=$(date +%Y%m%d_%H%M%S)
BK=/www/backup/admin_site_pre_${TS}.tgz
echo "=== 0. merge chunks ==="
cat /www/backup/admc_* > "$SRC" 2>/dev/null
[ -s "$SRC" ] || { echo "ABORT: merged empty"; echo "##FINISHED"; exit 1; }
GOT=$(sha256sum "$SRC" | awk '{print $1}'); SIZE=$(stat -c%s "$SRC")
echo "size=$SIZE expect=$EXPECT_SIZE"; echo "sha=$GOT"
if [ "$GOT" != "$EXPECT_SHA" ] || [ "$SIZE" != "$EXPECT_SIZE" ]; then echo "ABORT: mismatch"; echo "##FINISHED"; exit 1; fi
echo "=== 1. pre ==="; echo "old_entry=$(grep -o 'index-[A-Za-z0-9_-]*\.js' $APP/index.html | head -1)"
echo "=== 2. backup ==="; tar -czf "$BK" -C "$APP" index.html assets admin-assets 2>/dev/null; echo "backup=$BK size=$(stat -c%s "$BK" 2>/dev/null)"
echo "=== 3. extract (additive) ==="; tar -xzf "$SRC" -C "$APP"; echo "extract_rc=$?"
echo "=== 4. 权限 ==="
find "$APP/assets" -type d -exec chmod 755 {} \; 2>/dev/null
find "$APP/assets" -type f -exec chmod 644 {} \; 2>/dev/null
chmod 644 "$APP/index.html" 2>/dev/null
echo "entry_mode=$(stat -c%a $APP/assets/$EXPECT_ENTRY)"
echo "=== 5. verify ==="
echo "entry_in_html=$(grep -o 'index-[A-Za-z0-9_-]*\.js' $APP/index.html | head -1)"
echo "entry_http=$(curl -s -o /dev/null -w '%{http_code}' --max-time 8 -k -H 'Host: admin.recy.cc' https://127.0.0.1/assets/$EXPECT_ENTRY)"
echo "css_http=$(curl -s -o /dev/null -w '%{http_code}' --max-time 8 -k -H 'Host: admin.recy.cc' https://127.0.0.1/assets/$EXPECT_CSS)"
echo "entry_bytes=$(stat -c%s $APP/assets/$EXPECT_ENTRY)"
echo "entry_has_titlepad=$(grep -c 'titlePaddingLeft' $APP/assets/$EXPECT_ENTRY)"
echo "index_http=$(curl -s -o /dev/null -w '%{http_code}' --max-time 10 -H 'Host: admin.recy.cc' http://127.0.0.1/)"
echo "=== 6. cleanup ==="; rm -f /www/backup/admc_*; echo "backup_kept=$BK"; echo "##FINISHED"
