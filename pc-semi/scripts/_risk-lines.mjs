// 打印匹配某个正则的行号与内容。用法：node scripts/_risk-lines.mjs file "<regex source>"
import fs from 'node:fs';

const file = process.argv[2];
const src = process.argv[3] || ':[^;{}]*\\b\\d+(px|em|%)?\\s*[*]\\s*\\d+';
const re = new RegExp(src);
const lines = fs.readFileSync(file, 'utf8').split('\n');
let n = 0;
lines.forEach((l, i) => {
  if (re.test(l)) {
    n += 1;
    console.log(`${String(i + 1).padStart(5)}| ${l}`);
  }
});
console.log(`-- ${n} hit(s)  (regex: ${src})`);
