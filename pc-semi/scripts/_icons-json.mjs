import fs from 'node:fs';
const dir = 'node_modules/@icon-park/react';
const json = JSON.parse(fs.readFileSync(`${dir}/icons.json`, 'utf8'));
const list = Array.isArray(json) ? json : Object.keys(json);
console.log('icons.json entries', list.length);
console.log('sample', JSON.stringify(list.slice(0, 5)));

// es 目录里的组件文件
const es = fs.readdirSync(`${dir}/es`).filter((f) => f.endsWith('.js'));
console.log('es files', es.length, es.slice(0, 6));

const want = ['Truck', 'Refund', 'Time', 'Phone', 'Comment', 'Lock', 'Integral', 'Coupon', 'Package', 'Star', 'Goods', 'Ticket', 'Wallet', 'Return', 'Security', 'Mobile', 'Discount', 'ShoppingBag', 'PayCode', 'FileText'];
for (const w of want) {
  const hit = es.find((f) => f.toLowerCase() === `${w.toLowerCase()}.js`);
  console.log(w, hit ? 'YES' : '-');
}
console.log('--- contains ---');
for (const k of ['truck', 'refund', 'return', 'clock', 'time', 'phone', 'comment', 'lock', 'integral', 'coupon', 'ticket', 'point', 'goods', 'order']) {
  const hits = es.filter((f) => f.toLowerCase().includes(k));
  if (hits.length) console.log(k, ':', hits.slice(0, 10).join(' '));
}
