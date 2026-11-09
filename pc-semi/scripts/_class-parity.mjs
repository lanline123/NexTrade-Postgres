#!/usr/bin/env node
/**
 * 旧版 .vue 模板类名 → 新版 TSX 覆盖率。
 * 用法：node scripts/_class-parity.mjs <legacy.vue> <tsx...>
 * 输出：旧版模板里出现的 class 中，有多少在新版 TSX 的 className 里完全找不到。
 */
import fs from 'node:fs';

const args = process.argv.slice(2).map((a) => a.replace(/^["']+|["']+$/g, ''));
const legacyPath = args[0];
const tsxPaths = args.slice(1);

function unquote(s) {
  return String(s || '').replace(/^["']+|["']+$/g, '');
}

function templateOf(vue) {
  const m = vue.match(/<template>([\s\S]*?)\n<\/template>/);
  return m ? m[1] : vue;
}

function legacyClasses(vue) {
  const tpl = templateOf(vue);
  const set = new Set();
  for (const m of tpl.matchAll(/\s(?::)?class="([^"]*)"/g)) {
    let raw = m[1];
    raw = raw.replace(/\{\{[\s\S]*?\}\}/g, ' ');
    raw = raw.replace(/'[^']*'/g, (q) => ' ' + q.slice(1, -1) + ' ');
    raw = raw.replace(/`[^`]*`/g, ' ');
    for (const tok of raw.split(/[\s,()?:[\]{}'"`+]+/)) {
      if (/^-?[a-zA-Z][\w-]*$/.test(tok) && tok.length > 1 && !/^(true|false|null|undefined)$/.test(tok)) set.add(tok);
    }
  }
  return set;
}

function tsxClasses(files) {
  const set = new Set();
  for (const f of files) {
    if (!fs.existsSync(f)) { console.log(`!! 缺文件 ${f}`); continue; }
    const src = fs.readFileSync(f, 'utf8');
    for (const m of src.matchAll(/className=(?:"([^"]*)"|\{`([\s\S]*?)`\}|\{([\s\S]*?)\n\s*\})/g)) {
      const raw = m[1] || m[2] || m[3] || '';
      for (const tok of raw.split(/[\s'"`+$&|?:[\]{}()]+/)) {
        if (/^-?[a-zA-Z][\w-]*$/.test(tok) && tok.length > 1) set.add(tok);
      }
    }
  }
  return set;
}

const legacy = fs.readFileSync(legacyPath, 'utf8');
const want = legacyClasses(legacy);
const have = tsxClasses(tsxPaths);
const missing = [...want].filter((c) => !have.has(c)).sort();

console.log(`旧版模板类 ${want.size} 个 / 新版 TSX 类 ${have.size} 个`);
console.log(`命中 ${want.size - missing.length} / ${want.size}`);
console.log(missing.length ? `缺失(${missing.length}): ${missing.join(' ')}` : '全部命中');
