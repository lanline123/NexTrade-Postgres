#!/usr/bin/env node
/**
 * 把某个文件从 marker 起到文件末尾整体换掉，新内容从 tailFile 读。
 * 用法：node scripts/_splice.mjs <file> <tailFile> <marker...>
 * marker 由后面所有参数拼成（避免一个长字符串带引号）。
 */
import fs from 'node:fs';

const args = process.argv.slice(2).map((a) => a.replace(/^["']+|["']+$/g, ''));
const [file, tailFile, ...rest] = args;
let marker = rest.join(' ');
if (marker === '@marker') marker = fs.readFileSync('.tmp-legacy/marker.txt', 'utf8').replace(/\s+$/, '');
if (!file || !tailFile || !marker) {
  console.error('用法: node scripts/_splice.mjs <file> <tailFile> <marker...>');
  process.exit(1);
}
const src = fs.readFileSync(file, 'utf8');
const idx = src.indexOf(marker);
if (idx < 0) {
  console.error(`找不到 marker: ${marker}`);
  process.exit(1);
}
const second = src.indexOf(marker, idx + 1);
if (second >= 0) {
  console.error(`marker 出现多次（${idx} / ${second}），请写更长一点`);
  process.exit(1);
}
const tail = fs.readFileSync(tailFile, 'utf8');
const next = src.slice(0, idx) + tail;
fs.writeFileSync(file, next, 'utf8');
console.log(`${file}: ${src.length} -> ${next.length} bytes（从偏移 ${idx} 起整体替换）`);
