#!/usr/bin/env node
/**
 * 检查生成的 scss 里有没有“不带 . 开头”的顶层选择器（会污染全局）。
 * 用法：node scripts/_scss-global-risk.mjs <file>
 */
import fs from 'node:fs';
const file = (process.argv[2] || '').replace(/^["']+|["']+$/g, '');
const src = fs.readFileSync(file, 'utf8').replace(/\/\*[\s\S]*?\*\//g, ' ');
const lines = src.split(/\r?\n/);
let depth = 0;
const risky = [];
for (const line of lines) {
  const trimmed = line.trim();
  if (!trimmed) continue;
  // 收集每条 “{” 之前的最后一段作为选择器
  const m = trimmed.match(/^(.*)\{\s*$/);
  if (m) {
    const sel = m[1].trim();
    if (sel && !sel.startsWith('@')) {
      const first = sel.split(',')[0].trim();
      const safe =
        first.startsWith('.') ||
        first.startsWith('&') ||
        first.startsWith('#') ||
        first.startsWith('[') ||
        first.startsWith(':') ||
        /^[a-z]/.test(first) === false;
      if (!safe) risky.push(`${depth} ${first}`);
      // 标签选择器 + 类名的组合，如 div.x 也算安全，但单标签不安全
      if (/^[a-z][a-z0-9]*$/.test(first)) risky.push(`${depth} TAG ${first}`);
    }
  }
  depth += (line.match(/{/g) || []).length;
  depth -= (line.match(/}/g) || []).length;
}
if (!risky.length) {
  console.log('没有裸标签 / 全局选择器');
} else {
  console.log(`可疑 ${risky.length} 条（前 40）：`);
  risky.slice(0, 40).forEach((r) => console.log('  ' + r));
}
