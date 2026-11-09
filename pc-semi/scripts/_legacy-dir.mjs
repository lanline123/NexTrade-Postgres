import fs from 'node:fs';
import path from 'node:path';

const root = 'D:/nextrade/NexTrade/java-b2b2c-overseas-v5.8.61/view/pc';
const target = process.argv[2] || '';
const dir = path.join(root, target);
if (!fs.existsSync(dir)) {
  console.log('NO DIR ' + dir);
  process.exit(0);
}
const walk = (d) => {
  for (const name of fs.readdirSync(d)) {
    const full = path.join(d, name);
    const stat = fs.statSync(full);
    if (stat.isDirectory()) walk(full);
    else console.log(path.relative(root, full).replace(/\\/g, '/') + '  ' + stat.size + 'B');
  }
};
walk(dir);
