#!/usr/bin/env node
/**
 * 装修模块类名审计：逐个旧版 Pc*.vue 抽模板里的 class 令牌，对照新版 modules.tsx。
 * 用法：node scripts/_decorate-audit.mjs [<legacyDir>] [<tsxFile>]
 */
import fs from 'node:fs';
import path from 'node:path';

const strip = (s) => (s || '').replace(/^["']+|["']+$/g, '');
const legacyDir = strip(process.argv[2]) || 'D:/nextrade/NexTrade/java-b2b2c-overseas-v5.8.61/view/pc/decorate/components/pc';
const tsxFile = strip(process.argv[3]) || 'src/decorate/components/modules.tsx';

const STOP = new Set([
  'v-if','v-else','v-for','v-show','v-model','true','false','null','undefined','index','item','key','value',
  'length','includes','String','Number','Boolean','Math','module','title','style','type','id','name','url','link',
  'of','in','as','new','this','return','props','data','list','active','open','show','is','to','and','or','not',
]);
const isClassToken = (t) => /^[a-z][A-Za-z0-9_-]*$/.test(t) && !STOP.has(t) && t.length > 1;

function templateOf(source) {
  const m = source.match(/<template>([\s\S]*)<\/template>/);
  return m ? m[1] : source;
}

function classTokensOf(template) {
  const tokens = new Set();
  const push = (raw) => {
    // :class 里可能是一整段表达式，逐段切
    raw.split(/[^A-Za-z0-9_-]+/).forEach((t) => {
      const trimmed = t.replace(/^[-]+|[-]+$/g, '');
      if (isClassToken(trimmed)) tokens.add(trimmed);
    });
  };
  const re = /(?:^|\s)(:?class)\s*=\s*"([^"]*)"/g;
  let m;
  while ((m = re.exec(template))) {
    const value = m[2];
    if (m[1] === 'class') {
      value.split(/\s+/).filter(Boolean).forEach((t) => tokens.add(t));
    } else {
      // :class="..." → 取所有字符串字面量 + 裸类名
      const lits = value.match(/'[^']*'|"[^"]*"/g) || [];
      lits.forEach((l) => push(l.replace(/^['"]|['"]$/g, '')));
      push(value.replace(/'[^']*'|"[^"]*"/g, ' '));
    }
  }
  // 静态 class= 也可能有对象式
  const re2 = /(?:^|\s)class\s*=\s*'([^']*)'/g;
  while ((m = re2.exec(template))) {
    m[1].split(/\s+/).filter(Boolean).forEach((t) => tokens.add(t));
  }
  return tokens;
}

const tsx = fs.readFileSync(tsxFile, 'utf8');
const tsxTokens = new Set();
{
  const re = /class(?:Name)?\s*=\s*(?:"([^"]*)"|'([^']*)'|\{`([^`]*)`\})/g;
  let m;
  while ((m = re.exec(tsx))) {
    const raw = m[1] || m[2] || m[3] || '';
    raw.replace(/\$\{[^}]*\}/g, ' ').split(/[^A-Za-z0-9_-]+/).forEach((t) => {
      if (t.length > 1) tsxTokens.add(t);
    });
    const lits = raw.match(/'[^']*'|"[^"]*"/g) || [];
    lits.forEach((l) => l.replace(/^['"]|['"]$/g, '').split(/\s+/).filter(Boolean).forEach((t) => tsxTokens.add(t)));
  }
  // 模板串里 ${cond ? 'x' : ''} 的字符串
  const reInner = /'([a-z][A-Za-z0-9_-]*)'/g;
  while ((m = reInner.exec(tsx))) {
    if (m[1].length > 2) tsxTokens.add(m[1]);
  }
}

const files = fs.readdirSync(legacyDir).filter((f) => f.endsWith('.vue')).sort();
let grandMissing = 0;
let grandTotal = 0;
const report = [];
for (const file of files) {
  const source = fs.readFileSync(path.join(legacyDir, file), 'utf8');
  const tokens = classTokensOf(templateOf(source));
  const list = [...tokens].sort();
  const missing = list.filter((t) => !tsxTokens.has(t));
  grandTotal += list.length;
  grandMissing += missing.length;
  report.push({ file, total: list.length, missing });
}
report.sort((a, b) => b.missing.length - a.missing.length);
for (const row of report) {
  console.log(`\n### ${row.file}  类名 ${row.total}，缺 ${row.missing.length}`);
  if (row.missing.length) console.log('  ' + row.missing.join('  '));
}
console.log(`\n== 合计：类名 ${grandTotal}，缺 ${grandMissing}`);
