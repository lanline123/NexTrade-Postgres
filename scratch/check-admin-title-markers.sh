#!/bin/bash
# 只读：核对已上线的 admin 入口包里「标题契约收尾」标记（assets 不可外链，只能服务器侧核）
set -u
APP=/www/wwwroot/admin.recy.cc
E=$(grep -o 'index-[A-Za-z0-9_-]*\.js' $APP/index.html | head -1)
C=$(grep -o 'index-[A-Za-z0-9_-]*\.css' $APP/index.html | head -1)
echo "entry=$E css=$C"
echo "entry_bytes=$(stat -c%s $APP/assets/$E 2>/dev/null)"
echo "entry_legacy_recommend_h3=$(grep -c 'dec-pc-recommend-product__title' $APP/assets/$E)  # 期望 0"
echo "entry_has_titleblock=$(grep -c '标题设置' $APP/assets/$E)  # 期望 >0"
echo "entry_has_imagenav=$(grep -c 'pc-image-nav' $APP/assets/$E)  # 期望 >0"
echo "entry_has_shared_title=$(grep -c 'dec-module-title' $APP/assets/$E)  # 期望 >0"
echo "index_http=$(curl -s -o /dev/null -w '%{http_code}' --max-time 10 -H 'Host: admin.recy.cc' http://127.0.0.1/)"
echo "##FINISHED"
