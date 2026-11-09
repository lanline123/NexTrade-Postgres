// 找某个组件文件名：node scripts/_lf.mjs MemberTopMenu
import fs from 'node:fs';
import path from 'node:path';

const root = 'D:/nextrade/NexTrade/java-b2b2c-overseas-v5.8.61/view/pc';
const needle = (process.argv[2] || '').toLowerCase();
const hits = [];
function walk(dir) {
  let entries;
  try {
    entries = fs.readdirSync(dir, { withFileTypes: true });
  } catch {
    return;
  }
  for (const entry of entries) {
    if (entry.name === 'node_modules' || entry.name === '.nuxt' || entry.name === '.git') continue;
    const full = path.join(dir, entry.name);
    if (entry.isDirectory()) walk(full);
    else if (entry.name.toLowerCase().includes(needle)) hits.push(full);
  }
}
walk(root);
console.log(hits.length, 'hit(s)');
for (const hit of hits.slice(0, 20)) console.log(hit.replace(root, '').replace(/\\/g, '/'));
