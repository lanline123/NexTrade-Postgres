import fs from 'node:fs';
import path from 'node:path';

const root = 'out';
const files = fs.readdirSync(root).filter((name) => fs.statSync(path.join(root, name)).isFile());
console.log('out/ root files:', files.join(', '));

const cssDir = path.join(root, '_next/static/css');
const sheets = fs.readdirSync(cssDir).map((name) => [name, fs.readFileSync(path.join(cssDir, name), 'utf8')]);
const all = sheets.map(([, text]) => text).join('\n');
const needles = [
  ['button.search-submit', 'search button reset'],
  ['.menu-list .cc-collapse-item__toggle', 'help accordion'],
  ['.menu-list .min-menu .item-menu', 'help accordion articles'],
  ['.article-side.article-side a', 'news sidebar padding override'],
  ['.article-side .left_more', 'news sidebar chevron'],
  ['issue-menu', 'OLD help menu (should be absent)'],
];
for (const [needle, label] of needles) {
  console.log(`${all.includes(needle) ? 'YES' : ' - '}  ${label} (${needle})`);
}
console.log('css files:', sheets.map(([name]) => name).join(', '));
