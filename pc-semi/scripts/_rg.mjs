#!/usr/bin/env node
/**
 * 目录级字面量搜索（可用工具，不参与构建）
 * 用法: node scripts/_rg.mjs <目录> <字面量> [最多条数] [扩展名,扩展名]
 * 例子: node scripts/_rg.mjs src "pc-co-" 30 .tsx,.ts
 * 说明: 逐文件 readFileSync + includes，不做正则、不解析 AST；
 *       默认扫 .tsx .ts .jsx .js .vue .scss .css .md。
 */
import { readdirSync, readFileSync, statSync } from 'node:fs';
import { join, extname } from 'node:path';

const strip = (s) => (s || '').replace(/^["']|["']$/g, '');
const dir = strip(process.argv[2]);
const needle = strip(process.argv[3]);
const max = Number(strip(process.argv[4]) || 50);
const exts = (strip(process.argv[5]) || '.tsx,.ts,.jsx,.js,.vue,.scss,.css,.md')
  .split(',')
  .map((e) => (e.startsWith('.') ? e : `.${e}`));

if (!dir || !needle) {
  console.error('用法: node scripts/_rg.mjs <目录> <字面量> [最多条数] [扩展名]');
  process.exit(2);
}

const skipDirs = new Set(['node_modules', '.next', 'out', '.git', '.tmp-legacy', 'dist']);
let hits = 0;
let files = 0;

function walk(path) {
  if (hits >= max) return;
  let entries = [];
  try {
    entries = readdirSync(path, { withFileTypes: true });
  } catch {
    return;
  }
  for (const entry of entries) {
    if (hits >= max) return;
    const full = join(path, entry.name);
    if (entry.isDirectory()) {
      if (skipDirs.has(entry.name)) continue;
      walk(full);
      continue;
    }
    if (!exts.includes(extname(entry.name))) continue;
    let text = '';
    try {
      if (statSync(full).size > 4 * 1024 * 1024) continue;
      text = readFileSync(full, 'utf8');
    } catch {
      continue;
    }
    files += 1;
    if (!text.includes(needle)) continue;
    const lines = text.split(/\r?\n/);
    for (let i = 0; i < lines.length && hits < max; i += 1) {
      if (!lines[i].includes(needle)) continue;
      hits += 1;
      console.log(`${full}:${i + 1}: ${lines[i].trim().slice(0, 160)}`);
    }
  }
}

walk(dir);
console.log(`-- ${needle} : ${hits} 处（扫了 ${files} 个文件，上限 ${max}）`);
