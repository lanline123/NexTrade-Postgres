/**
 * 把一个 .vue 的 template 骨架（只保留标签行/带 class 的行）写到文件，方便分页读。
 * 用法: node scripts/_tpl2file.mjs <vue> <out> [from] [to]
 */
import fs from 'node:fs';

const argv = process.argv.slice(2).map((a) => a.replace(/^["']+|["']+$/g, ''));
const [file, out, fromRaw, toRaw] = argv;
const from = Number(fromRaw || 1);
const to = Number(toRaw || 100000);
if (!file || !out) {
  console.error('用法: node scripts/_tpl2file.mjs <vue> <out> [from] [to]');
  process.exit(1);
}
const lines = fs.readFileSync(file, 'utf8').split('\n');
const res = [];
let base = -1;
for (let i = 0; i < lines.length; i += 1) {
  const n = i + 1;
  if (n < from) continue;
  if (n > to) break;
  const raw = lines[i].replace(/\r$/, '');
  const t = raw.trim();
  if (!t) continue;
  if (t === '</template>' && raw.match(/^ */)[0].length === 0) break;
  const isOpen = t.startsWith('<');
  const isClose = t.startsWith('</') || t.startsWith('>');
  const hasClass = t.includes('class=');
  if (!isOpen && !isClose && !hasClass) continue;
  const indent = raw.match(/^ */)[0].length;
  if (base < 0) base = indent;
  const lvl = Math.max(0, Math.round((indent - base) / 4));
  res.push(`${String(n).padStart(4)} ${'  '.repeat(Math.min(lvl, 20))}${t.replace(/\s+/g, ' ')}`);
}
fs.mkdirSync('.tmp-legacy', { recursive: true });
fs.writeFileSync(out, res.join('\n'), 'utf8');
console.log(`${out}: ${res.length} lines`);
