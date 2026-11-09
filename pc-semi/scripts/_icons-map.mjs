import fs from 'node:fs';
const map = fs.readFileSync('node_modules/@icon-park/react/es/map.js', 'utf8');
const names = [...new Set([...map.matchAll(/(?:as ([A-Z][A-Za-z0-9]+))/g)].map((m) => m[1]))];
console.log('exports', names.length);
const want = ['Truck', 'Refund', 'Time', 'Phone', 'Comment', 'Lock', 'Integral', 'Coupon', 'Star', 'Goods', 'Ticket', 'Wallet', 'Return', 'Security', 'Mobile', 'Discount', 'Shopping', 'PayCode', 'FileText', 'Order', 'Progress'];
for (const w of want) console.log(w.padEnd(10), names.includes(w) ? 'YES' : '-');
console.log('--- contains ---');
for (const k of ['truck', 'refund', 'return', 'time', 'phone', 'comment', 'lock', 'integral', 'coupon', 'ticket', 'point', 'goods', 'pay', 'order', 'safe', 'protect']) {
  const hits = names.filter((n) => n.toLowerCase().includes(k));
  if (hits.length) console.log(k, ':', hits.slice(0, 10).join(' '));
}
