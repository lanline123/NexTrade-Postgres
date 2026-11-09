// 找 semi-icons 实际安装位置与 star 图标导出：node scripts/_semi-icons.mjs
import fs from 'node:fs';
import path from 'node:path';

const roots = [
  '../node_modules/@douyinfe',
  'node_modules/@douyinfe',
  '../../node_modules/@douyinfe',
];
for (const root of roots) {
  const dir = path.resolve(process.cwd(), root, 'semi-icons');
  if (!fs.existsSync(dir)) continue;
  console.log('found', dir);
  const pkg = JSON.parse(fs.readFileSync(path.join(dir, 'package.json'), 'utf8'));
  console.log('version', pkg.version, 'main', pkg.main, 'module', pkg.module, 'types', pkg.types);
  for (const sub of ['lib', 'es']) {
    const icons = path.join(dir, sub, 'icons');
    if (!fs.existsSync(icons)) continue;
    const names = fs.readdirSync(icons).filter((name) => /star/i.test(name)).slice(0, 20);
    console.log(sub, 'icons-with-star:', names.join(', ') || '(none)');
  }
  for (const rel of ['lib/es/index.d.ts', 'es/index.d.ts', 'lib/index.d.ts']) {
    const file = path.join(dir, rel);
    if (!fs.existsSync(file)) continue;
    const text = fs.readFileSync(file, 'utf8');
    const hits = text.split(/\r?\n/).filter((line) => /Star/i.test(line)).slice(0, 12);
    console.log(rel, hits.length ? hits.join('\n') : '(no Star lines)');
  }
  break;
}
