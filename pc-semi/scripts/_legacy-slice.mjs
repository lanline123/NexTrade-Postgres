#!/usr/bin/env node
/**
 * 把旧版某个文件的行区间落地成项目内的临时文本，方便用 build_file_read 完整读（不受工具输出截断影响）。
 * 用法：node scripts/_legacy-slice.mjs pages/member/account/CardManagement.vue 1 100
 * 输出：.tmp-legacy/<文件名>__<from>-<to>.txt
 */
import fs from 'node:fs';
import path from 'node:path';

const ROOT = 'D:/nextrade/NexTrade/java-b2b2c-overseas-v5.8.61/view/pc';
const [relRaw, fromRaw, toRaw] = process.argv.slice(2);
const rel = (relRaw || '').replace(/^["']+|["']+$/g, '');
if (!rel) {
  console.error('用法: node scripts/_legacy-slice.mjs <相对 view/pc 的路径> [from] [to]');
  process.exit(2);
}
const file = path.join(ROOT, rel);
if (!fs.existsSync(file)) {
  console.error(`文件不存在: ${file}`);
  process.exit(1);
}
const lines = fs.readFileSync(file, 'utf8').split(/\r?\n/);
const from = Math.max(1, Number(fromRaw || 1));
const to = Math.min(lines.length, Number(toRaw || from + 200));
const outDir = '.tmp-legacy';
fs.mkdirSync(outDir, { recursive: true });
const name = `${path.basename(rel)}__${from}-${to}.txt`;
const outPath = path.join(outDir, name);
const body = lines.slice(from - 1, to).map((line, index) => `${String(from + index).padStart(5)} | ${line}`).join('\n');
fs.writeFileSync(outPath, body, 'utf8');
console.log(`${outPath}  (${to - from + 1} 行，共 ${lines.length} 行)`);
