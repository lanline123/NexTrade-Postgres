#!/usr/bin/env node
/**
 * 一次性：搜索页重排 = 换 import + 从 return( 起整体替换成旧版结构。
 */
import fs from 'node:fs';

const file = 'app/search/page.tsx';
let s = fs.readFileSync(file, 'utf8');
const before = s.length;

s = s.replace(
  "import SearchBrandFilter from '@/components/SearchBrandFilter';\nimport SearchAllFilterPanel from '@/components/SearchAllFilterPanel';\nimport { SearchSideLayout, SearchClassicLayout, SearchModernLayout, type SearchLayoutContext } from '@/components/SearchLayoutVariants';",
  "import SearchAllFilterPanel from '@/components/SearchAllFilterPanel';\nimport SearchSortBar from '@/components/SearchSortBar';"
);

const marker = '  return (\n    <>\n      <PcHeader config={header as any} />\n';
const idx = s.indexOf(marker);
if (idx < 0) {
  console.error('找不到 marker');
  process.exit(1);
}
const tail = fs.readFileSync('.tmp-legacy/search-tail.txt', 'utf8');
s = s.slice(0, idx) + tail;

fs.writeFileSync(file, s, 'utf8');
console.log(`${file}: ${before} -> ${s.length} 字节`);
console.log(s.includes("from '@/components/SearchSortBar'") ? 'import SearchSortBar OK' : 'import SearchSortBar MISSING');
console.log(s.includes('SearchLayoutVariants') ? 'layout imports STILL THERE' : 'layout imports removed');
console.log(s.includes('mp-search-filter') ? 'legacy markup OK' : 'legacy markup MISSING');
