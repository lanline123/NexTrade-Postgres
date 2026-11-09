#!/usr/bin/env node
/**
 * 正则搜索单个文本文件（项目内或绝对路径）。
 * 用法：node scripts/_grep.mjs <file> <regex> [maxHits=80]
 */
import fs from 'node:fs';

const args = process.argv.slice(2).map((a) => a.replace(/^["']+|["']+$/g, ''));
const [file, pattern, maxRaw] = args;
const max = Number(maxRaw || 80);
if (!file || !pattern) {
  console.error('用法: node scripts/_grep.mjs <file> <regex> [maxHits]');
  process.exit(1);
}
const lines = fs.readFileSync(file, 'utf8').split('\n');
let re;
try {
  re = new RegExp(pattern);
} catch (e) {
  console.error('正则错误: ' + e.message);
  process.exit(1);
}
let hits = 0;
const out = [];
for (let i = 0; i < lines.length; i++) {
  if (re.test(lines[i])) {
    hits++;
    if (out.length < max) out.push(`${String(i + 1).padStart(5)}| ${lines[i].slice(0, 220)}`);
  }
}
console.log(out.join('\n'));
console.log(`-- ${pattern} : ${hits} 处（文件 ${lines.length} 行）`);
