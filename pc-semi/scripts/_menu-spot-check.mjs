import fs from 'node:fs';
const file = 'out/article/issue/index.html';
const h = fs.readFileSync(file, 'utf8');
const show = (label, re) => {
  const m = re.exec(h);
  console.log(`${label}:`, m ? m[0].replace(/\s+/g, ' ').slice(0, 300) : '(not found)');
};
show('menu-list', /<nav class="menu-list"[^>]*>/);
show('first item', /<div class="cc-collapse-item[^"]*">[\s\S]{0,300}/);
console.log('toggle buttons:', (h.match(/cc-collapse-item__toggle/g) || []).length);
console.log('aria-expanded count:', (h.match(/aria-expanded="/g) || []).length);
console.log('issue-menu leftovers:', (h.match(/issue-menu/g) || []).length);
