import fs from 'node:fs';
import path from 'node:path';

const root = process.cwd();
const cssDir = path.join(root, 'out', '_next', 'static', 'css');
const needles = [
  '.page-info-body',
  '.order-table-info',
  '.receipt-header',
  '.receipt-body',
  '.receipt-item',
  '.receipt-footer',
  '.receipt-footer-it',
  '.order-ul',
  '.virtualsample',
  '.extraskudata',
  '.img-border',
  '.product-cell__name',
  '.order-left',
  '.order-right',
  '.presale-steps',
  '.sub-title'
];

let all = '';
const files = fs.existsSync(cssDir) ? fs.readdirSync(cssDir).filter((f) => f.endsWith('.css')) : [];
for (const f of files) all += fs.readFileSync(path.join(cssDir, f), 'utf8');

console.log(`css files: ${files.length}, total bytes: ${all.length}`);
let missing = 0;
for (const n of needles) {
  const hit = all.includes(n);
  if (!hit) missing += 1;
  console.log(`${hit ? 'YES' : 'MISSING'}  ${n}`);
}

const page = path.join(root, 'out', 'member', 'order', 'detail', 'index.html');
if (fs.existsSync(page)) {
  const html = fs.readFileSync(page, 'utf8');
  const title = (html.match(/<title>([^<]*)<\/title>/) || [])[1] || '(none)';
  console.log(`detail html: ${html.length} bytes, title="${title}", has order-table-info=${html.includes('order-table-info')}`);
} else {
  console.log('detail html: MISSING');
}
console.log(missing === 0 ? 'RESULT: all legacy detail classes present' : `RESULT: ${missing} missing`);
