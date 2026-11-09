#!/usr/bin/env node
/**
 * 打印某行区间里所有以 { 结尾的选择器行（可用工具）
 * 用法: node scripts/_sels.mjs <文件> <起始行> <结束行>
 */
import { readFileSync } from 'node:fs';

const strip = (s) => (s || '').replace(/^["']|["']$/g, '');
const file = strip(process.argv[2]);
const from = Number(strip(process.argv[3]));
const to = Number(strip(process.argv[4]));
const lines = readFileSync(file, 'utf8').split(/\r?\n/);
for (let i = from - 1; i < Math.min(to, lines.length); i += 1) {
  const text = lines[i].trimEnd();
  if (/\{$/.test(text) || /,\s*$/.test(text)) console.log(`${i + 1}| ${text.trim().slice(0, 140)}`);
}
