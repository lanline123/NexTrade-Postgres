import fs from 'node:fs';
const dir = 'node_modules/@icon-park/react/es/icons';
const files = new Set(fs.readdirSync(dir).filter((f) => f.endsWith('.js')).map((f) => f.replace(/\.js$/, '')));
console.log('icons dir files', files.size);
for (const n of ['Truck', 'Return', 'Time', 'Timeline', 'Phone', 'Comment', 'Lock', 'Wallet', 'Coupon', 'Search', 'Down', 'TicketOne']) {
  console.log(n.padEnd(10), files.has(n) ? 'YES' : '-');
}
