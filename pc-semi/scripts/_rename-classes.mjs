#!/usr/bin/env node
/**
 * 按 旧串->新串 批量替换一个文件里的类名（一次性脚本，改完即可删）
 */
import fs from 'node:fs';

const strip = (s) => (s || '').replace(/^["']|["']$/g, '');
const file = strip(process.argv[2]);
let text = fs.readFileSync(file, 'utf8');
const before = text;

const MAP = [
  ['pc-search-filter-trigger-wrap', 'sort-filter-right__inner'],
  ['pc-search-filter-trigger__badge', 'sort-filter__badge'],
  ['pc-search-filter-trigger', 'sort-item sort-filter__trigger'],
  ['pc-search-filter-panel__mask', 'sort-filter-panel-mask'],
  ['pc-search-filter-panel__header', 'sort-filter-panel__header'],
  ['pc-search-filter-panel__title', 'sort-filter-panel__title'],
  ['pc-search-filter-panel__close', 'sort-filter-panel__close'],
  ['pc-search-filter-panel__tabs', 'sort-filter-panel__tabs'],
  ['pc-search-filter-panel__tab', 'sort-filter-panel__tab'],
  ['pc-search-filter-panel__body', 'sort-filter-panel__body'],
  ['pc-search-filter-panel__section-title', 'sort-filter-panel__section-title'],
  ['pc-search-filter-panel__section', 'sort-filter-panel__section'],
  ['pc-search-filter-panel__options', 'sort-filter-panel__options'],
  ['pc-search-filter-panel__option', 'sort-filter-panel__option'],
  ['pc-search-filter-panel__toggle', 'sort-filter-panel__toggle'],
  ['pc-search-filter-panel__empty', 'sort-filter-panel__empty'],
  ['pc-search-filter-panel__footer', 'sort-filter-panel__footer'],
  ['pc-search-filter-panel__reset', 'sort-filter-panel__reset'],
  ['pc-search-filter-panel', 'sort-filter-panel']
];

// 长串优先，避免前缀被先替换掉
MAP.sort((a, b) => b[0].length - a[0].length);
const counts = [];
for (const [from, to] of MAP) {
  const parts = text.split(from);
  counts.push(`${from} -> ${to} : ${parts.length - 1} 处`);
  text = parts.join(to);
}
// 布尔修饰类：旧版用 active，不用 is-active
text = text.replace(/ is-active/g, ' active');

fs.writeFileSync(file, text, 'utf8');
console.log(counts.join('\n'));
console.log(`\n${file}: ${before.length} -> ${text.length} 字节`);
