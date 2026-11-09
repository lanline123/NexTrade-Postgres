#!/usr/bin/env node
/** 列出 modules.tsx 里所有 function 声明名 + 行号，方便定位可复用组件与 hook。 */
import fs from 'node:fs';

const file = process.argv[2] || 'src/decorate/components/modules.tsx';
const needle = (process.argv[3] || '').toLowerCase();
const lines = fs.readFileSync(file, 'utf8').split('\n');

const rows = [];
for (let index = 0; index < lines.length; index += 1) {
  const text = lines[index];
  const match = /^(?:export\s+)?(?:async\s+)?function\s+([A-Za-z0-9_]+)/.exec(text.trim()) || /^const\s+([A-Za-z0-9_]+)\s*=\s*(?:\(|function|async)/.exec(text.trim());
  if (!match) continue;
  if (needle && !match[1].toLowerCase().includes(needle)) continue;
  rows.push(`${String(index + 1).padStart(5)}  ${match[1]}`);
}
console.log(`### ${file}: ${rows.length} 个声明${needle ? `（含 "${needle}"）` : ''}`);
console.log(rows.join('\n'));
