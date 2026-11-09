// 检查图片与样式：node scripts/_star-assets.mjs
import fs from 'node:fs';
import path from 'node:path';

const legacyPng = 'D:/nextrade/NexTrade/java-b2b2c-overseas-v5.8.61/view/pc/assets/images/common/icon-star.png';
console.log('legacy sprite:', fs.existsSync(legacyPng) ? `${fs.statSync(legacyPng).size} B` : 'MISSING');

for (const rel of ['public/assets/images/common/icon-star.png', 'public/assets/images/common']) {
  const full = path.resolve(process.cwd(), rel);
  if (!fs.existsSync(full)) {
    console.log(rel, '-> MISSING');
    continue;
  }
  const stat = fs.statSync(full);
  console.log(rel, stat.isDirectory() ? `dir with ${fs.readdirSync(full).length} entries` : `${stat.size} B`);
}

for (const rel of ['app/globals.scss', 'app/pc-global-utils.scss']) {
  const text = fs.readFileSync(path.resolve(process.cwd(), rel), 'utf8');
  console.log(rel, 'has .icon-star:', text.includes('.icon-star'), '| has icon-star.png:', text.includes('icon-star.png'));
}
