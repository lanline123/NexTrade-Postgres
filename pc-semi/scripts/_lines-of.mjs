#!/usr/bin/env node
/**
 * 列出文件里匹配正则的行（正则里可以写空格，因为整段作为 argv[3] 传入）。
 * 用法：node scripts/_lines-of.mjs <file> <regex> [max]
 */
import fs from 'node:fs';
const strip = (s) => (s || '').replace(/^["']+|["']+$/g, '');
const file = strip(process.argv[2]);
const pattern = strip(process.argv[3]);
const max = Number(strip(process.argv[4]) || 200) || 200;
if (!file || !pattern) {
  console.error('用法: node scripts/_lines-of.mjs <file> <regex> [max]');
  process.exit(1);
}
const lines = fs.readFileSync(file, 'utf8').split(/\r?\n/);
let re;
try {
  re = new RegExp(pattern);
} catch (e) {
  console.error('正则错误: ' + e.message);
  process.exit(1);
}
let n = 0;
for (let i = 0; i < lines.length; i += 1) {
  if (re.test(lines[i])) {
    console.log(`${String(i + 1).padStart(5)}| ${lines[i].slice(0, 240)}`);
    n += 1;
    if (n >= max) {
      console.log(`-- 到上限 ${max} 停止`);
      break;
    }
  }
}
console.log(`-- ${n} 行命中（文件共 ${lines.length} 行）`);
