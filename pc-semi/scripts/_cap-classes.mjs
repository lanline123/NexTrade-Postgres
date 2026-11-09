import fs from 'node:fs';
const p = process.argv[2] || '.tmp-r28/cap-preview2/capture.json';
const needles = (process.argv[3] || 'goods-wrap,item,pic,info,name,desc,price-box,newprice,oldprice,head-title,contnt,left,right').split(',');
const j = JSON.parse(fs.readFileSync(p, 'utf8'));
const out = j.outline || [];
const count = new Map();
for (const o of out) {
  const cls = typeof o.class === 'string' ? o.class : '';
  for (const n of needles) {
    if (cls.split(/\s+/).includes(n)) count.set(n, (count.get(n) || 0) + 1);
  }
}
console.log(`outline=${out.length}`);
for (const n of needles) console.log(`  ${n.padEnd(12)} ${count.get(n) || 0}`);
const simple = out.filter((o) => typeof o.class === 'string' && /price-box|goods-wrap/.test(o.class));
console.log('\nprice-box / goods-wrap boxes:');
for (const s of simple.slice(0, 12)) console.log(' ', JSON.stringify(s).slice(0, 240));
