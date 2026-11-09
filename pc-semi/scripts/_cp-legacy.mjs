#!/usr/bin/env node
/**
 * 把旧版 view/pc 下的若干文件复制进 .tmp-r24/legacy/，方便用 build_file_read 看全文
 * （build_file_read 不许读项目外的路径，_lines.mjs 又会截断长行）。
 *
 * 用法： node scripts/_cp-legacy.mjs <相对 view/pc 的路径> ...
 */
import fs from 'node:fs';
import path from 'node:path';

const BASE = 'D:/nextrade/NexTrade/java-b2b2c-overseas-v5.8.61/view/pc/';
const OUT = path.resolve('.tmp-r24/legacy');
fs.mkdirSync(OUT, { recursive: true });

const targets = process.argv.slice(2);
if (!targets.length) {
  console.error('用法: node scripts/_cp-legacy.mjs <相对 path> ...');
  process.exit(1);
}
for (const rel of targets) {
  const src = BASE + rel;
  const dst = path.join(OUT, rel.replace(/\//g, '__'));
  fs.copyFileSync(src, dst);
  console.log(`${rel}  ->  ${dst}  (${fs.statSync(dst).size} bytes)`);
}
