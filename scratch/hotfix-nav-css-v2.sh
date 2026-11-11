#!/bin/bash
set -u
CSS=/www/wwwroot/battery.recy.cc/.next/static/css/a3166646ccc0de4a.css
U=/_next/static/css/a3166646ccc0de4a.css
KEY=a3166646ccc0de4a.css
DIR=/www/server/nginx/proxy_cache_dir
TS=$(date +%Y%m%d_%H%M%S)
BK=/www/backup/pc_css_${TS}.css
CBK=/www/backup/nginx_cache_${TS}

echo "=== 0. recent PC deploy bundles ==="
ls -lt /www/backup/pc_pre_*.tgz 2>/dev/null | head -5
echo "before_size=$(stat -c%s "$CSS" 2>/dev/null) marker=$(grep -c 'dec-cat-image-size' "$CSS" 2>/dev/null)"

echo "=== 1. backup source ==="
cp "$CSS" "$BK" || { echo "ABORT: backup failed"; echo "##FINISHED"; exit 1; }
echo "backup=$BK size=$(stat -c%s "$BK")"

echo "=== 2. append hotfix (idempotent) ==="
if ! grep -q 'dec-cat-image-size' "$CSS"; then
cat >> "$CSS" <<'HOTFIX'
.dec-category-nav-wrapper .seckill_list_wrap{--dec-cat-image-size:180px}.dec-category-nav-wrapper .seckill_list_wrap .dec-swiper-nav{top:calc(var(--dec-cat-image-size)/2);margin-top:-24px;z-index:6;background-color:rgba(0,0,0,.32);font-size:20px;line-height:1}.dec-category-nav-wrapper .seckill_list_wrap .dec-swiper-nav:hover{background-color:rgba(0,0,0,.5)}.dec-category-nav-wrapper .seckill_list_wrap .dec-swiper-nav--prev{left:0}.dec-category-nav-wrapper .seckill_list_wrap .dec-swiper-nav--next{right:0}
HOTFIX
fi
echo "after_size=$(stat -c%s "$CSS") marker=$(grep -c 'dec-cat-image-size' "$CSS")"

echo "=== 3. purge nginx proxy_cache for this css ==="
FILES=$(grep -rl "$KEY" "$DIR" 2>/dev/null || true)
CNT=$(printf '%s\n' "$FILES" | grep -c '/')
mkdir -p "$CBK"
MOVED=0
for f in $FILES; do
  [ -f "$f" ] || continue
  if mv "$f" "$CBK/$(printf '%s' "$f" | tr '/' '_')"; then MOVED=$((MOVED+1)); fi
done
echo "matched=$CNT moved=$MOVED cache_backup=$CBK"

echo "=== 4. verify 3005 direct ==="
A=$(curl -s --max-time 12 "http://127.0.0.1:3005$U"); echo "chars=${#A} marker=$(printf '%s' "$A" | grep -c 'dec-cat-image-size')"
echo "=== 5. verify public URL (server side) ==="
B=$(curl -s --max-time 12 "https://battery.recy.cc$U"); echo "chars=${#B} marker=$(printf '%s' "$B" | grep -c 'dec-cat-image-size')"
echo "##FINISHED"
