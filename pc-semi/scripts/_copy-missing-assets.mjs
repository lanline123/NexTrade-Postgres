#!/usr/bin/env node
/**
 * 把旧版仓库里、被生成的 SCSS 引用但 public/ 下缺失的图片原样拷过来（不改字节、不改路径）。
 */
import fs from 'node:fs';
import path from 'node:path';

const legacyRoot = 'D:/nextrade/NexTrade/java-b2b2c-overseas-v5.8.61/view/pc/assets/images';
const rels = [
  'common/empty.gif',
  'common/sprite.png',
  'common/sprite@2x.png',
  'category/search.png',
  'flow/freight-icon.png',
  'index/group_bg.png',
];

for (const rel of rels) {
  const from = path.join(legacyRoot, rel);
  const to = path.join('public/assets/images', rel);
  if (!fs.existsSync(from)) {
    console.log(`源缺失，跳过 ${rel}`);
    continue;
  }
  if (fs.existsSync(to)) {
    console.log(`已存在，跳过 ${rel}`);
    continue;
  }
  fs.mkdirSync(path.dirname(to), { recursive: true });
  fs.copyFileSync(from, to);
  console.log(`拷贝 ${rel} -> ${to} (${fs.statSync(to).size} B)`);
}
