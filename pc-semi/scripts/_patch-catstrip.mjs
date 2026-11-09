#!/usr/bin/env node
/** 一次性：给右箭头 div 补上 className（上次手写漏了） */
import fs from 'node:fs';
const file = 'src/components/SearchCategoryStrip.tsx';
let s = fs.readFileSync(file, 'utf8');
if (!s.includes('filter-category__arrow--right')) {
  s = s.replace(
    /(\n\s*)onClick=\{\(\) => scrollByStep\(1\)\}/,
    '$1className="filter-category__arrow filter-category__arrow--right"$1onClick={() => scrollByStep(1)}'
  );
  fs.writeFileSync(file, s, 'utf8');
}
console.log(s.includes('filter-category__arrow--right') ? 'right arrow className OK' : 'MISSING');
