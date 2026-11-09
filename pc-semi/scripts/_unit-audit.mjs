#!/usr/bin/env node
/**
 * 统计样式里对根字号敏感的单位：rem / em 的数值用法。
 * 用来判断把 html{font-size} 改成旧版的 12px 会不会连带缩放版面。
 * 用法：node scripts/_unit-audit.mjs
 */
import fs from 'node:fs';
import path from 'node:path';

const ROOTS = ['app', 'src'];
const EXT = new Set(['.scss', '.css', '.tsx', '.ts']);

const files = [];
const walk = (dir) => {
  for (const e of fs.readdirSync(dir, { withFileTypes: true })) {
    if (e.name === 'node_modules' || e.name === 'out' || e.name === '.next') continue;
    const p = path.join(dir, e.name);
    if (e.isDirectory()) walk(p);
    else if (EXT.has(path.extname(e.name))) files.push(p);
  }
};

for (const r of ROOTS) if (fs.existsSync(r)) walk(r);

const remRe = /(?<![\w.-])(\d*\.?\d+)rem\b/g;
const emRe = /(?<![\w.-])(\d*\.?\d+)em\b/g;

let remTotal = 0;
let emTotal = 0;
const remHits = [];
const emHits = [];

for (const f of files) {
  if (f.endsWith('semi.css')) continue;
  const text = fs.readFileSync(f, 'utf8');
  const lines = text.split(/\r?\n/);
  lines.forEach((line, i) => {
    for (const m of line.matchAll(remRe)) {
      remTotal += 1;
      if (remHits.length < 30) remHits.push(`${f}:${i + 1}  ${m[0]}  |  ${line.trim().slice(0, 100)}`);
    }
    for (const m of line.matchAll(emRe)) {
      if (m[0] === 'em' || m[0].endsWith('em')) {
        emTotal += 1;
        if (emHits.length < 20) emHits.push(`${f}:${i + 1}  ${m[0]}  |  ${line.trim().slice(0, 100)}`);
      }
    }
  });
}

console.log(`扫描 ${files.length} 个文件`);
console.log(`rem 用法: ${remTotal}`);
for (const h of remHits) console.log('  ' + h);
console.log(`em 用法（含 -webkit-line-clamp 之类的无关词风险）: ${emTotal}`);
for (const h of emHits) console.log('  ' + h);
