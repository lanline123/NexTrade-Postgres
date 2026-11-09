#!/usr/bin/env node
/**
 * 统计一组字面量在若干文件里的出现次数。用法：node scripts/_count-in.mjs <needleFile> <file...>
 * needleFile 每行一个，忽略空行和 # 注释。
 */
import fs from 'node:fs';
const strip = (s) => (s || '').replace(/^["']+|["']+$/g, '');
const needleFile = strip(process.argv[2]);
const targets = process.argv.slice(3).map(strip).filter(Boolean);
const needles = fs.readFileSync(needleFile, 'utf8')
  .split(/\r?\n/)
  .map((l) => l.trim())
  .filter((l) => l && !l.startsWith('#'));
const sources = targets.map((f) => ({ f, text: fs.readFileSync(f, 'utf8') }));
let zero = 0;
for (const needle of needles) {
  const counts = sources.map((s) => s.text.split(needle).length - 1);
  const total = counts.reduce((a, b) => a + b, 0);
  if (total === 0) zero += 1;
  console.log(`${String(total).padStart(4)}  ${needle}`);
}
console.log(`-- ${needles.length} 个，其中 ${zero} 个 0 命中`);
