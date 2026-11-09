import fs from 'node:fs';
import path from 'node:path';

// 1) 静态快照里 /member/order/list 是什么（未登录会被中间件重定向，这里只做记录）
const page = 'out/member/order/list/index.html';
if (fs.existsSync(page)) {
  const h = fs.readFileSync(page, 'utf8');
  const title = (/<title>([^<]*)<\/title>/.exec(h) || [, ''])[1];
  console.log(`${page}: ${h.length} bytes, title="${title}", has memberOrder=${h.includes('memberOrder')}`);
} else {
  console.log(`${page} -> MISSING`);
}

// 2) 编译后的 CSS 里是否有本批新增的旧版 class
const cssDir = 'out/_next/static/css';
const needles = [
  '.memberOrder_top',
  '.memberOrder_middle',
  '.memberOrder_shop',
  '.memberOrder_bot',
  '.tig-tabs .tab',
  '.activeert',
  '.no-result',
  '.el-page',
  '.deleteWeigh',
  '.presale-text',
  '.joining-tag',
];
const files = fs.existsSync(cssDir) ? fs.readdirSync(cssDir).filter((f) => f.endsWith('.css')) : [];
const hit = Object.fromEntries(needles.map((n) => [n, []]));
for (const f of files) {
  const css = fs.readFileSync(path.join(cssDir, f), 'utf8');
  for (const n of needles) if (css.includes(n)) hit[n].push(f);
}
console.log(`\nCSS files scanned: ${files.length}`);
for (const n of needles) console.log('  ', n.padEnd(20), hit[n].length ? `YES (${hit[n].join(', ')})` : 'NO');
