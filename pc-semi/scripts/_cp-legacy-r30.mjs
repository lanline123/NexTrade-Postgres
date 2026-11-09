#!/usr/bin/env node
/**
 * 第 30 轮：把旧版 view/pc 下的若干文件复制进 .tmp-r30/legacy/，
 * 方便用 build_file_read 看全文（build_file_read 不许读项目外的路径）。
 *
 * 用法： node scripts/_cp-legacy-r30.mjs <相对 view/pc 的路径> ...
 */
import fs from 'node:fs';
import path from 'node:path';

const BASE = 'D:/nextrade/NexTrade/java-b2b2c-overseas-v5.8.61/view/pc/';
const OUT = path.resolve('.tmp-r30/legacy');
fs.mkdirSync(OUT, { recursive: true });

const targets = process.argv
  .slice(2)
  .map((a) => a.replace(/^["']+|["']+$/g, ''))
  .filter(Boolean);
if (!targets.length) {
  console.error('用法: node scripts/_cp-legacy-r30.mjs <相对 path> ...');
  process.exit(1);
}
for (const rel of targets) {
  const src = BASE + rel;
  const dst = path.join(OUT, rel.replace(/\//g, '__'));
  try {
    fs.copyFileSync(src, dst);
    console.log(`OK   ${rel}  (${fs.statSync(dst).size} B)`);
  } catch (err) {
    console.log(`FAIL ${rel}  ${err.code}`);
  }
}
