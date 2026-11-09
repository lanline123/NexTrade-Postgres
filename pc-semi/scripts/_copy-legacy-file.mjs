#!/usr/bin/env node
/**
 * 拷单个旧版静态文件（不是目录）：
 *   node scripts/_copy-legacy-file.mjs assets/images/signs.png public/assets/images/signs.png
 *
 * 为什么需要：scripts/_copy-legacy-assets.mjs 的参数是"目录"，把文件路径传进去会在
 * public/ 下建出一个同名 **文件夹**（public/assets/images/signs.png/ ），浏览器拿不到图。
 * 会员中心 .no-result .sign 的雪碧图就是这么被漏掉的。
 */
import fs from 'node:fs';
import path from 'node:path';

const LEGACY_ROOT = 'D:/nextrade/NexTrade/java-b2b2c-overseas-v5.8.61/view/pc';

const [, , from, to] = process.argv;
if (!from || !to) {
  console.error('用法: node scripts/_copy-legacy-file.mjs <旧版相对路径> <项目内相对路径>');
  process.exit(2);
}

const src = path.join(LEGACY_ROOT, from);
const dest = path.join(process.cwd(), to);

if (!fs.existsSync(src) || !fs.statSync(src).isFile()) {
  console.error(`旧版文件不存在或不是文件: ${src}`);
  process.exit(1);
}

// 目标若是目录（之前误建的），先清掉
if (fs.existsSync(dest) && fs.statSync(dest).isDirectory()) {
  fs.rmSync(dest, { recursive: true, force: true });
  console.log(`清掉误建的目录 ${dest}`);
}

fs.mkdirSync(path.dirname(dest), { recursive: true });
fs.copyFileSync(src, dest);
console.log(`拷贝 ${from} → ${to}  (${fs.statSync(dest).size} 字节)`);
