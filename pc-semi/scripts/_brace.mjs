// 检查多个 SCSS 文件的花括号是否平衡（跳过注释与字符串）。用法：node scripts/_brace.mjs f1 f2 ...
import fs from 'node:fs';

for (const f of process.argv.slice(2)) {
  let s;
  try {
    s = fs.readFileSync(f, 'utf8');
  } catch (err) {
    console.log(`! ${err.code}  ${f}`);
    continue;
  }
  let depth = 0;
  let min = 0;
  let quote = null;
  let inBlockComment = false;
  let bad = null;
  const lines = s.split('\n');
  for (let i = 0; i < s.length; i += 1) {
    const c = s[i];
    const n = s[i + 1];
    if (inBlockComment) {
      if (c === '*' && n === '/') {
        inBlockComment = false;
        i += 1;
      }
      continue;
    }
    if (quote) {
      if (c === quote && s[i - 1] !== '\\') quote = null;
      continue;
    }
    if (c === '/' && n === '*') {
      inBlockComment = true;
      i += 1;
      continue;
    }
    if (c === '/' && n === '/') {
      while (i < s.length && s[i] !== '\n') i += 1;
      continue;
    }
    if (c === '"' || c === "'") {
      quote = c;
      continue;
    }
    if (c === '{') depth += 1;
    if (c === '}') {
      depth -= 1;
      if (depth < min) {
        min = depth;
        bad = s.slice(0, i).split('\n').length;
      }
    }
  }
  const ok = depth === 0 && min === 0;
  console.log(`${ok ? 'OK  ' : 'FAIL'}  final=${depth}  min=${min}@line ${bad ?? '-'}  ${lines.length}L  ${s.length}B  ${f}`);
}
