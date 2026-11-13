# -*- coding: utf-8 -*-
"""打包 admin-semi/dist 并生成服务器侧热更新脚本 dep-admin-navtitle.sh。

本轮内容（标题契约收尾，2026-11-13）：
  1) 图文导航 pc-image-nav 接通配置型标题（defaults.title + 「标题设置」面板）
  2) 推荐商品去掉写死 <h3> 双轨（改由 normalizeLegacyTitle 搬进契约，双端同口径）
  3) 秒杀前台不再读平铺 seckillTitle
  4) 热图绘制去掉 ShopTitle 冗余 rightText
  5) kind:'title' 的 panel hint 不再被 isSection 分支吞掉

流程与已验证的 pack-admin-rp.py 一致（HANDOVER §13.6）：
  合并分片 → 校验 sha/字节 → 备份 → 解包 staging → 铺 assets(additive) → 最后切 index.html → 校验。

用法：先在 admin-semi 跑 `npx vite build`（Node/pnpm 装在 Windows 侧），再执行本脚本。
"""
import hashlib
import io
import os
import re
import tarfile

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.dirname(HERE)
DIST = os.path.join(REPO, "admin-semi", "dist")
TGZ = os.path.join(HERE, "admin_navtitle.tgz")
OUT = os.path.join(HERE, "dep-admin-navtitle.sh")
PART_PREFIX = "antn_"

html = io.open(os.path.join(DIST, "index.html"), encoding="utf-8").read()
entry = re.search(r"assets/(index-[A-Za-z0-9_-]+\.js)", html).group(1)
css = re.search(r"assets/(index-[A-Za-z0-9_-]+\.css)", html).group(1)

members = []
for root, _dirs, files in os.walk(DIST):
    for fn in files:
        full = os.path.join(root, fn)
        rel = os.path.relpath(full, DIST).replace("\\", "/")
        members.append((rel, full))
members.sort()


def add_file(tf, full, arc):
    """显式写 0644（+ root:root），不用源文件的权限。

    坑（2026-11-13 线上白屏事故）：沙箱/挂载目录里的文件是 0700，tarfile.add 会把 0700 写进包；
    服务器上 cp 只对"已存在文件"保留原权限，**新出现**的文件就以 700 root 落地 → nginx（www/lighthouse）
    读不了 → 入口与 vendor chunk 全部 403，后台白屏。CI 打的包在 Linux 下 umask 022，不会踩这个坑。
    """
    ti = tf.gettarinfo(full, arcname=arc)
    ti.mode = 0o644
    ti.uid = ti.gid = 0
    ti.uname = ti.gname = 'root'
    with open(full, 'rb') as fh:
        tf.addfile(ti, fh)


with tarfile.open(TGZ, "w:gz") as tf:
    for rel, full in members:
        add_file(tf, full, rel)

size = os.path.getsize(TGZ)
sha = hashlib.sha256(open(TGZ, "rb").read()).hexdigest()

TEMPLATE = u"""#!/bin/bash
# Admin 热更新：标题契约收尾（图文导航接通配置型标题 + 推荐商品单路径 + 秒杀去平铺 + 面板 hint）
# 顺序（HANDOVER §13.6）：先铺 assets，最后切 index.html。
set -u

EXPECT_SHA={sha}
EXPECT_SIZE={size}
EXPECT_ENTRY={entry}
EXPECT_CSS={css}

APP=/www/wwwroot/admin.recy.cc
SRC=/www/backup/admin_navtitle.tgz
TS=$(date +%Y%m%d_%H%M%S)
BK=/www/backup/admin_site_pre_${{TS}}.tgz

echo "=== 0. merge chunks ==="
cat /www/backup/{prefix}* > "$SRC" 2>/dev/null
if [ ! -s "$SRC" ]; then echo "ABORT: merged empty"; echo "##FINISHED"; exit 1; fi
GOT=$(sha256sum "$SRC" | awk '{{print $1}}')
SIZE=$(stat -c%s "$SRC")
echo "size=$SIZE expect={size}"
echo "sha=$GOT"
if [ "$GOT" != "$EXPECT_SHA" ] || [ "$SIZE" != "{size}" ]; then
  echo "ABORT: mismatch"; echo "##FINISHED"; exit 1
fi

echo "=== 1. pre-state ==="
echo "old_entry=$(grep -o 'index-[A-Za-z0-9_-]*\\.js' $APP/index.html | head -1)"

echo "=== 2. backup ==="
tar -czf "$BK" -C "$APP" index.html assets admin-assets 2>/dev/null
echo "backup=$BK size=$(stat -c%s "$BK" 2>/dev/null)"

echo "=== 3. extract ==="
EXO=/www/backup/admin_navtitle_stage_${{TS}}
rm -rf "$EXO"; mkdir -p "$EXO"
tar -xzf "$SRC" -C "$EXO"
echo "extract_rc=$? staged=$(find "$EXO" -type f | wc -l)"

echo "=== 4. 铺 assets（additive，不删旧文件） ==="
find "$EXO/assets" -type f -exec cp -f {{}} "$APP/assets/" \\; 2>/dev/null
[ -d "$EXO/admin-assets" ] && cp -f "$EXO"/admin-assets/* "$APP/admin-assets/" 2>/dev/null
# 必须修权限：cp 只对"已存在文件"保留原权限，新文件会带源模式（曾因 0700 导致 nginx 403 白屏）
find "$APP/assets" -type d -exec chmod 755 {{}} \\; 2>/dev/null
find "$APP/assets" -type f -exec chmod 644 {{}} \\; 2>/dev/null
chmod 644 "$APP/index.html" 2>/dev/null
echo "entry_mode=$(stat -c%a "$APP/assets/$EXPECT_ENTRY" 2>/dev/null)"
echo "new_entry_present=$(ls -l "$APP/assets/$EXPECT_ENTRY" 2>/dev/null | wc -l)"
echo "new_css_present=$(ls -l "$APP/assets/$EXPECT_CSS" 2>/dev/null | wc -l)"

echo "=== 5. 切 index.html（最后一步） ==="
if [ ! -s "$APP/assets/$EXPECT_ENTRY" ]; then
  echo "ABORT: 新入口未就位"; echo "##FINISHED"; exit 1
fi
cp -f "$APP/index.html" "/www/backup/admin_index_before_${{TS}}.html"
cp -f "$EXO/index.html" "$APP/index.html"
echo "switch_rc=$?"
rm -rf "$EXO"

echo "=== 6. verify ==="
echo "entry_in_html=$(grep -o 'index-[A-Za-z0-9_-]*\\.js' $APP/index.html | head -1)"
echo "css_in_html=$(grep -o 'index-[A-Za-z0-9_-]*\\.css' $APP/index.html | head -1)"
C=$(curl -s -o /dev/null -w '%{{http_code}}' --max-time 8 -H 'Host: admin.recy.cc' "http://127.0.0.1/assets/$EXPECT_ENTRY")
CC=$(curl -s -o /dev/null -w '%{{http_code}}' --max-time 8 -H 'Host: admin.recy.cc' "http://127.0.0.1/assets/$EXPECT_CSS")
echo "entry_http=$C css_http=$CC"
# 烟雾标记：程序集里这几个字面量必须还在（说明是正常的 admin 入口，不是半截包）
echo "entry_has_titleblock=$(grep -c '标题设置' "$APP/assets/$EXPECT_ENTRY")"
echo "entry_has_imagenav=$(grep -c 'pc-image-nav' "$APP/assets/$EXPECT_ENTRY")"
# 负向标记：推荐商品写死的 h3 渲染已删除；此串只应出现在 CSS，出现在 JS 入口里说明改造没进包
echo "entry_legacy_recommend_h3=$(grep -c 'dec-pc-recommend-product__title' "$APP/assets/$EXPECT_ENTRY")"
echo "index_http=$(curl -s -o /dev/null -w '%{{http_code}}' --max-time 10 -H 'Host: admin.recy.cc' http://127.0.0.1/)"

echo "=== 7. 人工验收（不自动执行） ==="
echo "next: Playwright 注入 admin_access_token 打开 https://admin.recy.cc/?page=pcDecorateEditor&id=10023"
echo "check: 图文导航面板出现「标题设置」（显示标题/标题样式/对齐/内容/颜色/描述/背景/圆角），开启后画布出现标题栏"
echo "check: 秒杀面板「标题设置」下方出现提示文案（秒杀商品由营销-秒杀决定）"

echo "=== 8. cleanup ==="
rm -f /www/backup/{prefix}*
echo "backup_kept=$BK"
echo "##FINISHED"
"""

body = TEMPLATE.format(sha=sha, size=size, entry=entry, css=css, prefix=PART_PREFIX)
io.open(OUT, "w", encoding="utf-8", newline="\n").write(body)

print("entry=%s css=%s" % (entry, css))
print("tgz=%s size=%d sha256=%s" % (TGZ, size, sha))
print("dep=" + OUT)
print("files=%d" % len(members))
print("upload: python scratch/upload_chunks.py scratch/admin_navtitle.tgz %s" % PART_PREFIX.rstrip("_"))
