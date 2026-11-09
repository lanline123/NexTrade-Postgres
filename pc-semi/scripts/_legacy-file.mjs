#!/usr/bin/env node
/**
 * 按文件名（不含扩展名或含）在旧版 view/pc 里找文件。
 * 用法：node scripts/_legacy-file.mjs CommonPageContainer
 */
import fs from 'node:fs';
import path from 'node:path';

const ROOT = 'D:/nextrade/NexTrade/java-b2b2c-overseas-v5.8.61/view/pc';
const needle = (process.argv[2] || '').toLowerCase();
if (!needle) {
  console.error('用法: node scripts/_legacy-file.mjs <文件名片段>');
  process.exit(2);
}

const out = [];
const walk = (dir) => {
  for (const e of fs.readdirSync(dir, { withFileTypes: true })) {
    if (e.name === 'node_modules' || e.name === '.nuxt' || e.name === 'dist') continue;
    const p = path.join(dir, e.name);
    if (e.isDirectory()) walk(p);
    else if (e.name.toLowerCase().includes(needle)) out.push(p);
  }
};
walk(ROOT);

for (const f of out) {
  console.log(`${f.replace(ROOT + path.sep, '').replace(/\\/g, '/')}  (${fs.statSync(f).size})`);
}
console.log(`-- ${out.length} 个文件`);
