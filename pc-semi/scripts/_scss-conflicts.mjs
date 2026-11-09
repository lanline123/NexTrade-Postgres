#!/usr/bin/env node
/**
 * 只打印「选择器行里等于 / 包含某个类名」的顶层规则（选择器字符串 + 声明），
 * 用来对比同一个类名在多个组件里被摊平后的冲突。参数：<scss> <classNeedle> [limit]
 */
import fs from 'node:fs';

const file = process.argv[2];
const needle = process.argv[3];
const limit = Number(process.argv[4] || 60);
const lines = fs.readFileSync(file, 'utf8').split('\n');

const rules = [];
for (let index = 0; index < lines.length; index += 1) {
  const selector = lines[index].trim();
  if (!selector.endsWith('{') || selector.startsWith('@')) continue;
  if (!selector.includes(needle)) continue;
  // 只取紧跟在后面的第一层声明
  const body = [];
  let depth = 1;
  for (let inner = index + 1; inner < lines.length; inner += 1) {
    const text = lines[inner].trim();
    depth += (text.match(/\{/g) || []).length;
    depth -= (text.match(/\}/g) || []).length;
    if (depth <= 0) break;
    if (depth === 1 && text && !text.endsWith('{')) body.push(text);
  }
  rules.push({ line: index + 1, selector, body });
}

const grouped = new Map();
for (const rule of rules) {
  const key = `${rule.selector}||${rule.body.join(' ')}`;
  if (!grouped.has(key)) grouped.set(key, { ...rule, count: 1 });
  else grouped.get(key).count += 1;
}

console.log(`### ${file}：「${needle}」共 ${rules.length} 个规则块，去重后 ${grouped.size} 种`);
for (const rule of [...grouped.values()].slice(0, limit)) {
  console.log(`${rule.line}${rule.count > 1 ? ` (×${rule.count})` : ''}: ${rule.selector} → ${rule.body.join(' ')}`);
}
