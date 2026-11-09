#!/usr/bin/env node
/**
 * 从某个 SCSS 产物里，把「选择器以 X 开头/包含 X」的规则块连同其声明打印出来（带行号）。
 * 用法：node scripts/_scss-rules.mjs <scss 文件> <包含的选择器片段> [最大块数]
 */
import fs from 'node:fs';

const file = process.argv[2];
const needle = process.argv[3];
const limit = Number(process.argv[4] || 40);

const lines = fs.readFileSync(file, 'utf8').split('\n');
let printed = 0;
let depth = 0;
let capturing = false;
let buffer = [];

for (let index = 0; index < lines.length && printed < limit; index += 1) {
  const line = lines[index];
  const trimmed = line.trim();

  if (!capturing) {
    // 规则开始：这一行含 needle 且以 { 结尾（或下一行是 {）
    if (trimmed.includes(needle) && (trimmed.endsWith('{') || (lines[index + 1] || '').trim() === '{')) {
      capturing = true;
      buffer = [`${index + 1}: ${trimmed}`];
      depth = trimmed.endsWith('{') ? 1 : 0;
      if (depth === 0) {
        // 选择器单独一行，下一行是 {
        buffer.push(`${index + 2}: ${(lines[index + 1] || '').trim()}`);
        depth = 1;
        index += 1;
      }
      continue;
    }
    continue;
  }

  buffer.push(`${index + 1}: ${trimmed}`);
  depth += (trimmed.match(/\{/g) || []).length;
  depth -= (trimmed.match(/\}/g) || []).length;
  if (depth <= 0) {
    console.log(buffer.join('\n'));
    console.log('---');
    capturing = false;
    buffer = [];
    printed += 1;
  }
}

console.log(`### ${file}: 打印 ${printed} 个含 "${needle}" 的规则块`);
