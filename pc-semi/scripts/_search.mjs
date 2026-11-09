#!/usr/bin/env node
/**
 * 在单个文件里按字面量搜（区分大小写），打印行号。
 * 注意：命令执行器不剥引号，所以不要把参数写成 "..."；含空格的needle直接连写多个参数即可（会拼回去）。
 * 用法：node scripts/_search.mjs <文件> <要搜的文本...> [--max=10]
 */
import fs from 'node:fs';

const args = process.argv.slice(2);
const maxArg = args.find((a) => a.startsWith('--max='));
const rest = args.filter((a) => !a.startsWith('--max='));
const file = (rest[0] || '').replace(/^["']+|["']+$/g, '');
const needleParts = rest.slice(1).map((a) => a.replace(/^["']+|["']+$/g, ''));
if (!file || !needleParts.length) {
  console.error('用法: node scripts/_search.mjs <文件> <要搜的文本...> [--max=10]');
  process.exit(2);
}
const needle = needleParts.join(' ');
const max = maxArg ? Number(maxArg.slice(6)) : 10;

const lines = fs.readFileSync(file, 'utf8').split(/\r?\n/);
let hits = 0;
lines.forEach((line, i) => {
  if (hits >= max) return;
  if (line.includes(needle)) {
    hits += 1;
    console.log(`${String(i + 1).padStart(5)} | ${line.trim().slice(0, 220)}`);
  }
});
console.log(`-- needle=[${needle}] ${hits} 处（文件 ${lines.length} 行）`);
process.exit(hits > 0 ? 0 : 1);
