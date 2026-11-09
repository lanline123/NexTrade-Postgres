/**
 * 打印一个 .vue 文件 template 的骨架：只保留 < 开头 / class= / 结尾标签的行，缩进压平。
 * 用法: node scripts/_tpl.mjs <path-to-vue> [fromLine] [toLine] [maxLines]
 */
import fs from 'node:fs';

const argv = process.argv.slice(2).map((a) => a.replace(/^["']+|["']+$/g, ''));
const file = argv[0];
const from = Number(argv[1] || 1);
const to = Number(argv[2] || 100000);
const max = Number(argv[3] || 400);
if (!file) {
  console.error('用法: node scripts/_tpl.mjs <vue> [from] [to] [max]');
  process.exit(1);
}
const text = fs.readFileSync(file, 'utf8');
const lines = text.split('\n');
const out = [];
let baseIndent = -1;
for (let i = 0; i < lines.length; i += 1) {
  const n = i + 1;
  if (n < from) continue;
  if (n > to) break;
  const raw = lines[i].replace(/\r$/, '');
  const t = raw.trim();
  if (t === '</template>' && raw.match(/^\s*/)[0].length <= 0) break;
  if (!t) continue;
  const isOpen = t.startsWith('<');
  const isClose = t.startsWith('</') || t.startsWith('>');
  const hasClass = t.includes('class=');
  if (!isOpen && !isClose && !hasClass) continue;
  const indent = raw.match(/^ */)[0].length;
  if (baseIndent < 0) baseIndent = indent;
  const lvl = Math.max(0, Math.round((indent - baseIndent) / 4));
  out.push(`${String(n).padStart(4)} ${'  '.repeat(Math.min(lvl, 20))}${t.replace(/\s+/g, ' ').slice(0, 200)}`);
  if (out.length >= max) break;
}
console.log(out.join('\n'));
console.log(`-- ${out.length} line(s) of ${lines.length}`);
