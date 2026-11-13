#!/bin/bash
# Admin 上线：commit fb0cf15（GitHub CI admin-build-5）——标题左右边距 + 推荐商品默认值
set -u
EXPECT_SHA=e3b3c0d2addf2f8e4e77e1deeb535d793012911a1233e255d86aa0e573ba5bea
EXPECT_SIZE=4773497
EXPECT_ENTRY=index-D8JvhJZ8.js
EXPECT_CSS=index-CnNv8buu.css
APP=/www/wwwroot/admin.recy.cc
SRC=/www/backup/admin_ci5.tgz
TS=$(date +%Y%m%d_%H%M%S)
BK=/www/backup/admin_site_pre_${TS}.tgz
echo "=== 0. merge chunks ==="
cat /www/backup/admb_* > "$SRC" 2>/dev/null
[ -s "$SRC" ] || { echo "ABORT: merged empty"; echo "##FINISHED"; exit 1; }
GOT=$(sha256sum "$SRC" | awk '{print $1}'); SIZE=$(stat -c%s "$SRC")
echo "size=$SIZE expect=$EXPECT_SIZE"; echo "sha=$GOT"
if [ "$GOT" != "$EXPECT_SHA" ] || [ "$SIZE" != "$EXPECT_SIZE" ]; then echo "ABORT: mismatch"; echo "##FINISHED"; exit 1; fi
echo "=== 1. pre ==="; echo "old_entry=$(grep -o 'index-[A-Za-z0-9_-]*\.js' $APP/index.html | head -1)"
echo "=== 2. backup ==="; tar -czf "$BK" -C "$APP" index.html assets admin-assets 2>/dev/null; echo "backup=$BK size=$(stat -c%s "$BK" 2>/dev/null)"
echo "=== 3. extract (additive) ==="; tar -xzf "$SRC" -C "$APP"; echo "extract_rc=$?"
echo "=== 4. 权限（必须：新文件若带 0700 会让 nginx 403）==="
find "$APP/assets" -type d -exec chmod 755 {} \; 2>/dev/null
find "$APP/assets" -type f -exec chmod 644 {} \; 2>/dev/null
chmod 644 "$APP/index.html" 2>/dev/null
echo "entry_mode=$(stat -c%a $APP/assets/$EXPECT_ENTRY) owner=$(stat -c%U:%G $APP/assets/$EXPECT_ENTRY)"
echo "=== 5. verify ==="
echo "entry_in_html=$(grep -o 'index-[A-Za-z0-9_-]*\.js' $APP/index.html | head -1)"
echo "css_in_html=$(grep -o 'index-[A-Za-z0-9_-]*\.css' $APP/index.html | head -1)"
echo "entry_http=$(curl -s -o /dev/null -w '%{http_code}' --max-time 8 -k -H 'Host: admin.recy.cc' https://127.0.0.1/assets/$EXPECT_ENTRY)"
echo "css_http=$(curl -s -o /dev/null -w '%{http_code}' --max-time 8 -k -H 'Host: admin.recy.cc' https://127.0.0.1/assets/$EXPECT_CSS)"
echo "entry_bytes=$(stat -c%s $APP/assets/$EXPECT_ENTRY)"
echo "entry_has_titlepad=$(grep -c 'titlePaddingLeft' $APP/assets/$EXPECT_ENTRY)   # 期望 >0"
echo "entry_has_titlepadright=$(grep -c 'titlePaddingRight' $APP/assets/$EXPECT_ENTRY)  # 期望 >0"
echo "assets_total=$(ls -1 $APP/assets | wc -l)"
echo "index_http=$(curl -s -o /dev/null -w '%{http_code}' --max-time 10 -H 'Host: admin.recy.cc' http://127.0.0.1/)"
echo "=== 6. cleanup ==="; rm -f /www/backup/admb_*; echo "backup_kept=$BK"; echo "##FINISHED"
