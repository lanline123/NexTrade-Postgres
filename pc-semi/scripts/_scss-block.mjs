// 打印某个 .mp-* 包裹块的前若干行：node scripts/_scss-block.mjs mp-userinvoice-info 40
import fs from 'node:fs';
import path from 'node:path';

const needle = `.mp-${process.argv[2] || 'userinvoice-info'}`;
const limit = Number(process.argv[3] || 40);
const file = path.resolve(process.cwd(), 'app/member-pages.scss');
const lines = fs.readFileSync(file, 'utf8').split(/\r?\n/);
const start = lines.findIndex((line) => line.trim().startsWith(`${needle} {`) || line.trim().startsWith(`${needle}{`));
if (start < 0) {
  console.log('not found', needle, 'in', file);
  process.exit(0);
}
console.log(lines.slice(start, start + limit).join('\n'));
