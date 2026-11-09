#!/usr/bin/env node
/**
 * 编译一个 scss 并用正则在里面找选择器，打印命中行与偏移量。
 * 用法：node scripts/_scss-selectors.mjs app/decorate-source.scss "floor-wide__box .price"
 */
import * as sass from 'sass';

const file = (process.argv[2] || 'app/decorate-source.scss').replace(/^["']+|["']+$/g, '');
const needles = process.argv
  .slice(3)
  .join(' ')
  .split(',')
  .map((s) => s.trim())
  .filter(Boolean);
const { css } = sass.compile(file, { style: 'compressed', logger: sass.Logger.silent });
console.log(`compiled ${file} -> ${css.length} bytes`);
for (const needle of needles) {
  let from = 0;
  const hits = [];
  for (;;) {
    const at = css.indexOf(needle, from);
    if (at < 0) break;
    hits.push(at);
    from = at + needle.length;
    if (hits.length >= 20) break;
  }
  console.log(`### ${needle} — ${hits.length} 处 ${hits.slice(0, 8).join(', ')}`);
}
