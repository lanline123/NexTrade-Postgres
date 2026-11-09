#!/usr/bin/env node
// 行数 + 首行摘要：node scripts/_wc.mjs <file...>
// 注意：命令运行器会把引号原样带进 argv，这里先剥掉首尾引号。
import { readFileSync, statSync, existsSync } from 'node:fs';

for (const raw of process.argv.slice(2)) {
  const p = raw.replace(/^["']+|["']+$/g, '');
  if (!existsSync(p)) { console.log(`MISSING ${p}`); continue; }
  const s = readFileSync(p, 'utf8');
  const n = s.split('\n').length;
  const first = s.split('\n').find((l) => l.trim()) || '';
  console.log(`${String(n).padStart(5)}  ${(statSync(p).size + '').padStart(8)}  ${p}   ${first.slice(0, 90)}`);
}
