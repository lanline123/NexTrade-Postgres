import fs from 'node:fs';
const checks = [
  ['out/article/issue/index.html', ['zzfw-menu', 'left_info', 'svg', 'right-title zzfw']],
  ['out/article/news/list/index.html', ['article-side', 'min-title', 'left_more', 'min-list']],
];
for (const [file, needles] of checks) {
  if (!fs.existsSync(file)) {
    console.log(`${file} -> MISSING`);
    continue;
  }
  const h = fs.readFileSync(file, 'utf8');
  console.log(`${file} (${h.length} bytes)`);
  for (const n of needles) console.log('  ', n.padEnd(18), h.includes(n) ? 'YES' : '-');
  const m = /<aside class="article-side"[\s\S]{0,700}/.exec(h);
  if (m) console.log('   sidebar snippet:', m[0].replace(/\s+/g, ' ').slice(0, 600));
  const z = /<div class="zzfw-menu">[\s\S]{0,400}/.exec(h);
  if (z) console.log('   zzfw snippet:', z[0].replace(/\s+/g, ' ').slice(0, 420));
}
