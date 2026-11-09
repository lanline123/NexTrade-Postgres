import fs from 'node:fs';
import path from 'node:path';

const file = path.join(process.cwd(), 'app', 'member-pages.scss');
const lines = fs.readFileSync(file, 'utf8').split('\n');

let inHeader = true;
let hits = 0;
for (let i = 0; i < lines.length; i += 1) {
  const l = lines[i];
  if (inHeader) {
    if (l.trim() === '*/') inHeader = false;
    continue;
  }
  if (/:deep\(|@import|v-deep|scoped/.test(l)) {
    hits += 1;
    console.log(`${i + 1}: ${l.trim().slice(0, 160)}`);
  }
}
console.log(`--- ${hits} suspicious line(s) after header`);

// 括号配平检查，避免生成出无法解析的 SCSS
let depth = 0;
for (const l of lines) {
  for (const ch of l) {
    if (ch === '{') depth += 1;
    if (ch === '}') depth -= 1;
    if (depth < 0) {
      console.log('UNBALANCED: a } closes more than opened');
      process.exit(0);
    }
  }
}
console.log(`--- final brace depth: ${depth} (0 = balanced)`);
console.log(`--- ${lines.length} lines total`);
