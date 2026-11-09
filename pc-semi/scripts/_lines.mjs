// 打印指定行区间的原始行（调试用）。用法：node scripts/_lines.mjs <file> <from> <to>
import fs from 'node:fs';

const argv = process.argv.slice(2).map((a) => a.replace(/^["']+|["']+$/g, ''));
const file = argv[0];
const from = Number(argv[1] || 1);
const to = Number(argv[2] || 20);
const lines = fs.readFileSync(file, 'utf8').split('\n');
for (let i = from - 1; i < Math.min(to, lines.length); i += 1) {
  const raw = lines[i].replace(/\r$/, '');
  console.log(`${String(i + 1).padStart(4)}|${raw.slice(0, 240)}|${raw.length}`);
}
console.log(`-- total ${lines.length} lines`);
