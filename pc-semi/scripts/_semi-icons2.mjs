// 看 semi-icons 到底导出了哪些 Star：node scripts/_semi-icons2.mjs
import fs from 'node:fs';
import path from 'node:path';

const dir = path.resolve(process.cwd(), 'node_modules/@douyinfe/semi-icons');
for (const rel of ['lib/es/index.d.ts', 'lib/es/index.js', 'package.json']) {
  const file = path.join(dir, rel);
  if (!fs.existsSync(file)) {
    console.log(rel, '(missing)');
    continue;
  }
  const stat = fs.statSync(file);
  console.log(`--- ${rel} ${stat.size}B`);
  if (stat.size < 20000) {
    const text = fs.readFileSync(file, 'utf8');
    console.log(text.slice(0, 1200));
  } else {
    const text = fs.readFileSync(file, 'utf8');
    const hits = [...text.matchAll(/[^\n]*Star[^\n]*/g)].slice(0, 15).map((m) => m[0].slice(0, 160));
    console.log(hits.join('\n') || '(no Star)');
  }
}
const iconsDir = path.join(dir, 'lib/es/icons');
if (fs.existsSync(iconsDir)) {
  const names = fs.readdirSync(iconsDir).filter((n) => /star/i.test(n)).slice(0, 30);
  console.log('icons dir star files:', names.join(', ') || '(none)');
} else {
  console.log('no lib/es/icons dir');
}
