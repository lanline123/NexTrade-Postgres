import fs from 'node:fs';
const p = 'node_modules/@icon-park/react/es/all.js';
const alt = 'node_modules/@icon-park/react/es/index.js';
const file = fs.existsSync(p) ? p : alt;
if (!fs.existsSync(file)) {
  console.log('package not found at', p, alt);
  process.exit(0);
}
const t = fs.readFileSync(file, 'utf8');
const names = [...new Set([...t.matchAll(/as ([A-Z][A-Za-z0-9]+)/g)].map((m) => m[1]))];
console.log('total exports', names.length);
const want = ['Truck','Refund','Time','Phone','Comment','Lock','Integral','Coupon','Package','Star','CommentOne','Goods','Ticket','PayCode','Wallet','Notification','Return','Progress','Security','Mobile','Points','Discount','TicketOne','Car','ShoppingBag'];
for (const w of want) console.log(w, names.includes(w) ? 'YES' : '-');
console.log('--- candidates containing keyword ---');
for (const k of ['Truck','Refund','Return','Time','Phone','Comment','Lock','Integral','Coupon','Ticket','Point','Coupon','Security','Progress']) {
  const hits = names.filter((n) => n.toLowerCase().includes(k.toLowerCase()));
  if (hits.length) console.log(k, ':', hits.slice(0, 12).join(', '));
}
