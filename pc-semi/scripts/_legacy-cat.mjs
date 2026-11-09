import fs from 'node:fs';

const root = 'D:/nextrade/NexTrade/java-b2b2c-overseas-v5.8.61/view/pc';
const rel = process.argv[2];
const file = root + '/' + rel;
if (!fs.existsSync(file)) {
  console.log('ENOENT ' + file);
  process.exit(0);
}
const text = fs.readFileSync(file, 'utf8');
const lines = text.split(/\r?\n/);
const from = Number(process.argv[3] || 1);
const to = Number(process.argv[4] || 120);
for (let i = from - 1; i < Math.min(to, lines.length); i += 1) {
  console.log(String(i + 1).padStart(4) + '| ' + lines[i]);
}
