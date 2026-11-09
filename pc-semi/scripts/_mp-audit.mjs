// 会员页根节点 .mp-* 挂载情况：node scripts/_mp-audit.mjs
import fs from 'node:fs';
import path from 'node:path';

const root = path.resolve(process.cwd(), 'app/member');
const out = [];

function walk(dir) {
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const full = path.join(dir, entry.name);
    if (entry.isDirectory()) walk(full);
    else if (/\.(tsx|ts)$/.test(entry.name)) out.push(full);
  }
}
walk(root);

const rows = [];
for (const file of out) {
  const text = fs.readFileSync(file, 'utf8');
  const rel = path.relative(process.cwd(), file).replace(/\\/g, '/');
  const mp = [...text.matchAll(/mp-[a-z0-9-]+/g)].map((m) => m[0]);
  const uniq = [...new Set(mp)];
  rows.push({ rel, count: uniq.length, uniq });
}

rows.sort((a, b) => a.rel.localeCompare(b.rel));
for (const row of rows) {
  const flag = row.count === 0 ? 'NO-MP ' : 'mp    ';
  console.log(flag, row.rel, row.count ? row.uniq.slice(0, 4).join(',') : '');
}
console.log('total files', rows.length, 'without mp', rows.filter((r) => r.count === 0).length);
