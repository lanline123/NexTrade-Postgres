#!/usr/bin/env node
/**
 * 在 out/ 产物里按字面量搜索（CSS/HTML/JS），用于验证某条规则真的进了产物。
 * 注意：命令执行器不剥引号，调用时不要把参数写成 "..."（会被当成字面量）。
 * 用法：node scripts/_out-grep.mjs WenQuanYi
 */
import fs from 'node:fs';
import path from 'node:path';

const needle = process.argv[2];
if (!needle) {
  console.error('用法: node scripts/_out-grep.mjs <要搜的字面量>');
  process.exit(2);
}

const ROOT = 'out';
const files = [];
const walk = (dir, depth) => {
  if (depth > 6) return;
  for (const e of fs.readdirSync(dir, { withFileTypes: true })) {
    const p = path.join(dir, e.name);
    if (e.isDirectory()) walk(p, depth + 1);
    else if (/\.(css|html|js|json|txt|xml|webmanifest)$/.test(e.name)) files.push(p);
  }
};
if (!fs.existsSync(ROOT)) {
  console.error('out/ 不存在，先跑 npm run build');
  process.exit(2);
}
walk(ROOT, 0);

let hits = 0;
for (const f of files) {
  const text = fs.readFileSync(f, 'utf8');
  let idx = text.indexOf(needle);
  if (idx === -1) continue;
  hits += 1;
  console.log(`\n=== ${f}  (${fs.statSync(f).size} bytes)`);
  let shown = 0;
  while (idx !== -1 && shown < 3) {
    const start = Math.max(0, idx - 120);
    const end = Math.min(text.length, idx + needle.length + 160);
    console.log('  …' + text.slice(start, end).replace(/\s+/g, ' ') + '…');
    idx = text.indexOf(needle, idx + needle.length);
    shown += 1;
  }
}

console.log(`\n扫描 ${files.length} 个产物文件，命中文件 ${hits} 个`);
process.exit(hits > 0 ? 0 : 1);
