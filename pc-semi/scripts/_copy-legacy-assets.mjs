/**
 * 把旧版 Vue 里被 CSS 引用的背景图原样拷进 public/assets/images/。
 *
 * 只拷 app/*.scss 里真正 url(...) 引用到的文件，路径与旧站完全一致（旧站资源根 = /assets/）。
 * 不做任何缩放/转换。
 *
 * 用法: node scripts/_copy-legacy-assets.mjs
 */
import fs from 'node:fs';
import path from 'node:path';

const LEGACY = 'D:/nextrade/NexTrade/java-b2b2c-overseas-v5.8.61/view/pc';
const SRC_ROOT = path.join(LEGACY, 'assets', 'images');
const DEST_ROOT = 'public/assets/images';

const wanted = [
  'common/sprite-product.png',
  'product/product_title_bg.png',
  'product/price-bg.png',
  'product/pro-time-bg.png',
  'product/phone_color.png',
  'product/code-border.png',
  'product/video-playing.png',
  'product/sli_btn.png',
  'flow/ioc_arrow.gif',
  'user/coupon20160715.png',
  'user/slice-s924f6ceef1.png',
  'user/cicon.png',
  'common/odr_cfm_spt.png',
  'flow/tips-arrow-new.png',
  'flow/orderIcons.gif',
];

let copied = 0;
const missing = [];
const already = [];

for (const rel of wanted) {
  const src = path.join(SRC_ROOT, rel);
  const dest = path.join(DEST_ROOT, rel);
  if (!fs.existsSync(src)) {
    missing.push(rel);
    continue;
  }
  if (fs.existsSync(dest)) {
    already.push(rel);
    continue;
  }
  fs.mkdirSync(path.dirname(dest), { recursive: true });
  fs.copyFileSync(src, dest);
  copied += 1;
}

console.log(`copied ${copied}, already there ${already.length}, missing in legacy ${missing.length}`);
for (const m of missing) console.log('  MISSING', m);
for (const a of already) console.log('  EXISTED', a);
