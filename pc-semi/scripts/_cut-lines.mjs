#!/usr/bin/env node
/**
 * 删除若干行区间（可用工具，用于清理死样式）
 * 用法: node scripts/_cut-lines.mjs <文件> <起>:<止> [<起>:<止> ...]
 * 区间按 1 起始、含两端；从后往前删，行号不会错位。
 */
import { readFileSync, writeFileSync } from 'node:fs';

const strip = (s) => (s || '').replace(/^["']|["']$/g, '');
const file = strip(process.argv[2]);
const ranges = process.argv.slice(3).map((arg) => {
  const [a, b] = strip(arg).split(':');
  return [Number(a), Number(b)];
});

if (!file || !ranges.length) {
  console.error('用法: node scripts/_cut-lines.mjs <文件> <起>:<止> ...');
  process.exit(2);
}

const raw = readFileSync(file, 'utf8');
const lines = raw.split(/\r?\n/);
const before = lines.length;
const sorted = [...ranges].sort((x, y) => y[0] - x[0]);
for (const [from, to] of sorted) {
  if (!from || !to || from < 1 || to < from) {
    console.error(`区间不合法: ${from}:${to}`);
    process.exit(2);
  }
  lines.splice(from - 1, to - from + 1);
}
writeFileSync(file, lines.join('\n'), 'utf8');
console.log(`${file}: ${before} 行 -> ${lines.length} 行（删了 ${before - lines.length} 行，${sorted.length} 段）`);
