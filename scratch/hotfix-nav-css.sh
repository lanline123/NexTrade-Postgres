#!/bin/bash
set -u
CSS=/www/wwwroot/battery.recy.cc/.next/static/css/a3166646ccc0de4a.css
TS=$(date +%Y%m%d_%H%M%S)
BK=/www/backup/pc_css_a316_${TS}.css

echo "=== 0. precheck ==="
if [ ! -f "$CSS" ]; then echo "ABORT: css not found"; echo "##FINISHED"; exit 1; fi
echo "css=$CSS before_size=$(stat -c%s "$CSS")"
if grep -q 'dec-cat-image-size' "$CSS"; then echo "ABORT: already patched (idempotent guard)"; echo "##FINISHED"; exit 1; fi

echo "=== 1. backup ==="
cp "$CSS" "$BK" || { echo "ABORT: backup failed"; echo "##FINISHED"; exit 1; }
echo "backup=$BK size=$(stat -c%s "$BK")"

echo "=== 2. append hotfix ==="
cat >> "$CSS" <<'HOTFIX'
.dec-category-nav-wrapper .seckill_list_wrap{--dec-cat-image-size:180px}.dec-category-nav-wrapper .seckill_list_wrap .dec-swiper-nav{top:calc(var(--dec-cat-image-size)/2);margin-top:-24px;z-index:6;background-color:rgba(0,0,0,.32);font-size:20px;line-height:1}.dec-category-nav-wrapper .seckill_list_wrap .dec-swiper-nav:hover{background-color:rgba(0,0,0,.5)}.dec-category-nav-wrapper .seckill_list_wrap .dec-swiper-nav--prev{left:0}.dec-category-nav-wrapper .seckill_list_wrap .dec-swiper-nav--next{right:0}
HOTFIX
echo "after_size=$(stat -c%s "$CSS")"

echo "=== 3. verify ==="
MARK=$(grep -c 'dec-cat-image-size' "$CSS")
echo "marker_count=$MARK"
if [ "$MARK" -lt 1 ]; then
  echo "ABORT: patch missing -> ROLLBACK"
  cp "$BK" "$CSS"
  echo "rollback_size=$(stat -c%s "$CSS")"
  echo "##FINISHED"
  exit 1
fi
echo "tail=$(tail -c 160 "$CSS")"
echo "##FINISHED"
