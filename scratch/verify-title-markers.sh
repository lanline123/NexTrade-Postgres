#!/bin/bash
# 只读门禁：核对「固定标题 → 配置型标题」收尾是否进包/上线（HANDOVER §21.3）
# 用法：bash scratch/verify-title-markers.sh
# 判据：
#   admin 入口 JS 里 99 推荐商品写死 h3 的 className 必须为 0，且仍含「标题设置」「pc-image-nav」
#   PC 首页所有 chunk 里写死 h3 命中必须为 0，且图文导航所在 chunk 同时含共享标题栏类
set -u

ADMIN=https://admin.recy.cc/
PC=https://battery.recy.cc/
TMP=$(mktemp -d)
fail=0

echo "=== admin ==="
entry=$(curl -s --max-time 30 "$ADMIN" | grep -o 'assets/index-[A-Za-z0-9_-]*\.js' | head -1)
echo "entry=$entry"
if [ -z "$entry" ]; then echo "ABORT: 取不到 admin 入口"; exit 1; fi
code=$(curl -s --max-time 60 -o "$TMP/adm.js" -w '%{http_code}' "$ADMIN$entry")
adm_bytes=$(wc -c < "$TMP/adm.js")
echo "entry_http=$code entry_bytes=$adm_bytes"
# 注意：admin 的 assets 对外直接 GET 会被 nginx 403（146B 的 403 页），只有浏览器同源请求才放行。
# 拉不到就不能把「没命中」当成「标记合格」——改为明确跳过并提示服务器侧核对。
if [ "$code" != "200" ] || [ "$adm_bytes" -lt 500000 ]; then
  echo "SKIP admin 断言（资源不可外链，http=$code bytes=$adm_bytes）"
  echo "  请在服务器侧核对：grep -c 'dec-pc-recommend-product__title' /www/wwwroot/admin.recy.cc/$entry  # 期望 0"
  echo "                     grep -c '标题设置' /www/wwwroot/admin.recy.cc/$entry                         # 期望 >0"
  admin_skipped=1
else
  h3=$(grep -c 'dec-pc-recommend-product__title' "$TMP/adm.js" || true)
  tb=$(grep -c '标题设置' "$TMP/adm.js" || true)
  in=$(grep -c 'pc-image-nav' "$TMP/adm.js" || true)
  echo "entry_legacy_recommend_h3=$h3 (期望 0)"
  echo "entry_has_titleblock=$tb (期望 >0)"
  echo "entry_has_imagenav=$in (期望 >0)"
  [ "$h3" = "0" ] || fail=1
  [ "$tb" != "0" ] || fail=1
  [ "$in" != "0" ] || fail=1
fi

echo "=== PC ==="
curl -s --max-time 60 "$PC" -o "$TMP/home.html"
html_bytes=$(wc -c < "$TMP/home.html")
echo "html_bytes=$html_bytes"
grep -o '/_next/static/chunks/[^"]*\.js' "$TMP/home.html" | sort -u > "$TMP/list.txt"
echo "chunks=$(wc -l < "$TMP/list.txt")"
legacy=0; found=0
while read -r u; do
  f="$TMP/$(echo "$u" | md5sum | cut -c1-8).js"
  curl -s --max-time 25 "$PC${u#/}" -o "$f" 2>/dev/null || curl -s --max-time 25 "https://battery.recy.cc$u" -o "$f" || continue
  if grep -q 'dec-recommend-product__title' "$f"; then legacy=$((legacy+1)); fi
  if grep -q 'dec-pc-image-nav__content' "$f"; then
    echo "imgnav_chunk=$u has_shared_title=$(grep -c 'dec-module-title' "$f")"
    grep -q 'dec-module-title' "$f" && found=1
  fi
done < "$TMP/list.txt"
echo "legacy_h3_chunks=$legacy (期望 0)"
echo "imgnav_shared_title_found=$found (期望 1)"
[ "$legacy" = "0" ] || fail=1
[ "$found" = "1" ] || fail=1

rm -rf "$TMP"
if [ "$fail" != "0" ]; then echo "TITLE_MARKERS_FAIL"; elif [ "${admin_skipped:-0}" = "1" ]; then echo "TITLE_MARKERS_OK(pc) / admin 需服务器侧核对"; else echo "TITLE_MARKERS_OK"; fi
