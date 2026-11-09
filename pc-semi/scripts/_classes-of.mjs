#!/usr/bin/env node
/**
 * 列出旧版 .vue 模板里用到的 class 名字（去重、排序）。
 * 用法：node scripts/_classes-of.mjs <legacy.vue> [只保留包含该子串的名字]
 */
import fs from 'node:fs';

const raw = process.argv.slice(2).map((a) => a.replace(/^["']+|["']+$/g, ''));
const file = raw[0];
const filter = (raw[1] || '').toLowerCase();
const src = fs.readFileSync(file, 'utf8');
const m = src.match(/<template>([\s\S]*?)\n<\/template>/);
const tpl = m ? m[1] : src;
const set = new Set();
for (const cm of tpl.matchAll(/\s(?::)?class="([^"]*)"/g)) {
  let s = cm[1].replace(/\{\{[\s\S]*?\}\}/g, ' ');
  s = s.replace(/'[^']*'/g, (q) => ' ' + q.slice(1, -1) + ' ');
  s = s.replace(/`[^`]*`/g, ' ');
  for (const tok of s.split(/[\s,()?:[\]{}'"`+]+/)) {
    if (/^-?[a-zA-Z][\w-]*$/.test(tok) && tok.length > 1 && !/^(true|false|null|undefined)$/.test(tok)) set.add(tok);
  }
}
const all = [...set].sort();
const shown = filter ? all.filter((c) => c.toLowerCase().includes(filter)) : all;
console.log(`${all.length} 个类${filter ? `（含 "${filter}" 的 ${shown.length} 个）` : ''}`);
console.log(shown.join(' '));
