#!/usr/bin/env node
/**
 * 在文件里找 needle，打印上下文若干行。用法：node scripts/_ctx.mjs <file> <needle> [before] [after]
 */
import fs from 'node:fs';
const strip = (s) => (s || '').replace(/^["']+|["']+$/g, '');
const file = strip(process.argv[2]);
const needle = strip(process.argv[3]);
const before = Number(strip(process.argv[4]) || 3);
const after = Number(strip(process.argv[5]) || 6);
const lines = fs.readFileSync(file, 'utf8').split(/\r?\n/);
let hits = 0;
lines.forEach((line, i) => {
  if (line.includes(needle)) {
    hits += 1;
    console.log(`--- 命中 @${i + 1} ---`);
    for (let j = Math.max(0, i - before); j <= Math.min(lines.length - 1, i + after); j += 1) {
      console.log(`${String(j + 1).padStart(5)}| ${lines[j]}`);
    }
  }
});
console.log(`-- ${hits} 处命中`);
