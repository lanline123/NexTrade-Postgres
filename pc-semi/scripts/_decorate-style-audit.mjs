#!/usr/bin/env node
/**
 * 装修样式审计：抽旧版 Pc*.vue <style> 块里的顶层选择器，看新版 app/modules-source.scss 里有没有。
 * 用法：node scripts/_decorate-style-audit.mjs [<legacyDir>] [<scssFile>]
 */
import fs from 'node:fs';
import path from 'node:path';

const strip = (s) => (s || '').replace(/^["']+|["']+$/g, '');
const legacyDir = strip(process.argv[2]) || 'D:/nextrade/NexTrade/java-b2b2c-overseas-v5.8.61/view/pc/decorate/components/pc';
const scssFile = strip(process.argv[3]) || 'app/modules-source.scss';

const scss = fs.readFileSync(scssFile, 'utf8');

function styleBlocks(source) {
  const out = [];
  const re = /<style[^>]*>([\s\S]*?)<\/style>/g;
  let m;
  while ((m = re.exec(source))) out.push(m[1]);
  return out;
}

/** 从 less/scss 里抽选择器（去掉属性、@规则、值和声明） */
function selectorsOf(css) {
  const cleaned = css
    .replace(/\/\*[\s\S]*?\*\//g, ' ')
    .replace(/\/\/[^\n]*/g, ' ');
  const selectors = [];
  let depth = 0;
  let buf = '';
  for (const ch of cleaned) {
    if (ch === '{') {
      const line = buf.trim().split(/\r?\n/).pop()?.trim() || '';
      if (depth >= 0 && line && !line.startsWith('@') && line && /[.#&:\w\[]/.test(line)) {
        selectors.push({ sel: line, depth });
      }
      depth += 1;
      buf = '';
      continue;
    }
    if (ch === '}') {
      depth = Math.max(0, depth - 1);
      buf = '';
      continue;
    }
    if (ch === ';') {
      buf = '';
      continue;
    }
    buf += ch;
  }
  // 只要末段选择器（嵌套里最深那一段），并拆逗号
  const out = new Set();
  for (const { sel } of selectors) {
    sel.split(',').forEach((part) => {
      const last = part.trim().split(/\s+|>/).filter(Boolean).pop() || '';
      const token = last.replace(/::?[\w-]+(\([^)]*\))?/g, '').trim();
      if (token && token.length > 1 && !token.startsWith('@') && /^[.#]?[A-Za-z_&]/.test(token)) {
        out.add(token);
      }
    });
  }
  return out;
}

const files = fs.readdirSync(legacyDir).filter((f) => f.endsWith('.vue')).sort();
let total = 0;
let missingTotal = 0;
const rows = [];
for (const file of files) {
  const source = fs.readFileSync(path.join(legacyDir, file), 'utf8');
  const blocks = styleBlocks(source);
  if (!blocks.length) {
    rows.push({ file, total: 0, missing: [] });
    continue;
  }
  const sels = new Set();
  blocks.forEach((b) => selectorsOf(b).forEach((s) => sels.add(s)));
  const list = [...sels].sort();
  const missing = list.filter((s) => !scss.includes(s.replace(/^\./, '.')));
  total += list.length;
  missingTotal += missing.length;
  rows.push({ file, total: list.length, missing });
}
rows.sort((a, b) => b.missing.length - a.missing.length);
for (const row of rows) {
  console.log(`\n### ${row.file}  选择器 ${row.total}，缺 ${row.missing.length}`);
  if (row.missing.length) console.log('  ' + row.missing.join('  '));
}
console.log(`\n== 合计：选择器 ${total}，缺 ${missingTotal}`);
