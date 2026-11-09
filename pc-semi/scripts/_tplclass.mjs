/**
 * 只保留 class 的紧凑骨架：每行只留 标签名 + class 列表（+ 少量结构提示），截断到 96 字符。
 * 用法: node scripts/_tplclass.mjs <vue> <out>
 */
import fs from 'node:fs';

const argv = process.argv.slice(2).map((a) => a.replace(/^["']+|["']+$/g, ''));
const [file, out] = argv;
const lines = fs.readFileSync(file, 'utf8').split('\n');
const res = [];
let base = -1;
for (let i = 0; i < lines.length; i += 1) {
  const raw = lines[i].replace(/\r$/, '');
  const t = raw.trim();
  if (!t) continue;
  if (t === '</template>' && raw.match(/^ */)[0].length === 0) break;
  if (!t.startsWith('<') && !t.startsWith('>') && !t.includes('class=')) continue;
  const indent = raw.match(/^ */)[0].length;
  if (base < 0) base = indent;
  const lvl = Math.max(0, Math.round((indent - base) / 4));
  // 提取标签名
  const tagM = t.match(/^<\/?([A-Za-z][\w-]*)/);
  const tag = tagM ? (t.startsWith('</') ? '/' + tagM[1] : tagM[1]) : '';
  const classes = [];
  const re = /class="([^"]*)"/g;
  let m;
  while ((m = re.exec(t))) {
    const c = m[1].replace(/\{\{[\s\S]*?\}\}/g, '·').replace(/[{}]/g, '').trim();
    if (c) classes.push(c);
  }
  const dyn = /:class=/.test(t) ? ' :class' : '';
  const vif = /v-if=/.test(t) ? ' v-if' : '';
  const vfor = /v-for=/.test(t) ? ' v-for' : '';
  const body = [tag, ...classes.map((c) => '.' + c)].join('') + dyn + vif + vfor;
  if (!body) continue;
  res.push(`${'  '.repeat(Math.min(lvl, 18))}${body.slice(0, 96)}`);
}
fs.mkdirSync('.tmp-legacy', { recursive: true });
fs.writeFileSync(out, res.join('\n'), 'utf8');
console.log(`${out}: ${res.length} lines, ${fs.statSync(out).size} bytes`);
