#!/usr/bin/env node
/**
 * 扫 app/*.scss 里所有 url(/assets/...) 与 url(../../assets/...) 形式的本地图片引用，
 * 报告哪些在 public/ 下不存在（= 线上会 404 的空图）。
 */
import fs from 'node:fs';
import path from 'node:path';

const dirs = ['app'];
const files = [];
for (const dir of dirs) {
  for (const f of fs.readdirSync(dir)) {
    if (f.endsWith('.scss') || f.endsWith('.css')) files.push(path.join(dir, f));
  }
}

const missing = new Map();
const seen = new Set();
for (const file of files) {
  const text = fs.readFileSync(file, 'utf8');
  const re = /url\((['"]?)([^'")]+)\1\)/g;
  let m;
  while ((m = re.exec(text))) {
    const raw = m[2].trim();
    if (!raw || raw.startsWith('data:') || raw.startsWith('http')) continue;
    let local = null;
    if (raw.startsWith('/assets/')) local = path.join('public', raw.slice(1));
    else if (raw.startsWith('../../assets/')) local = path.join('public', raw.replace('../../', ''));
    if (!local) continue;
    const key = `${local}`;
    if (seen.has(key)) continue;
    seen.add(key);
    if (!fs.existsSync(local)) {
      if (!missing.has(local)) missing.set(local, []);
      missing.get(local).push(`${file}: ${raw}`);
    }
  }
}

console.log(`扫描 ${files.length} 个 scss，引用 ${seen.size} 个本地图片，缺 ${missing.size} 个`);
for (const [local, where] of [...missing.entries()].sort()) {
  console.log(`缺 ${local}   ← ${where.slice(0, 3).join(' | ')}`);
}
