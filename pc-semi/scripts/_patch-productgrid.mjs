#!/usr/bin/env node
/**
 * 一次性：给 ProductGrid 加 gridClassName，并换空态类名。
 */
import fs from 'node:fs';

const file = 'src/components/ProductGrid.tsx';
let s = fs.readFileSync(file, 'utf8');
const before = s.length;

s = s.replace(
  '  showShopTag?: boolean;\n}',
  '  showShopTag?: boolean;\n  /**\n   * 额外挂到网格根节点上的旧版类名。\n   * 旧版 `pages/search/components/CatGoodsList.vue` 的结构是\n   * `.goods-list__box > .goods-list-c > .wrap`，`.wrap` 才是网格本体\n   * （`grid-template-columns: repeat(var(--search-grid-columns, 5), minmax(0, 1fr))`），\n   * 所以搜索页把 wrap 传进来。\n   */\n  gridClassName?: string;\n}'
);

s = s.replace(
  'export default function ProductGrid({ products, showSales = false, showShopTag = false }: ProductGridProps) {',
  'export default function ProductGrid({ products, showSales = false, showShopTag = false, gridClassName }: ProductGridProps) {'
);

s = s.replace(
  '      className="product-card-grid"',
  '      className={`product-card-grid${gridClassName ? ` ${gridClassName}` : \'\'}`}'
);

// 旧版 CatGoodsList 的空态是 .search-empty（不是新版自造的 .pc-list-empty）
s = s.replace(
  '<div className="pc-list-empty">{t(\'没有找到商品\')}</div>',
  '<div className="search-empty"><span className="search-empty__text">{t(\'没有找到商品\')}</span></div>'
);

fs.writeFileSync(file, s, 'utf8');
console.log(`${file}: ${before} -> ${s.length} 字节`);
console.log(s.includes('gridClassName') ? 'gridClassName OK' : 'gridClassName MISSING');
console.log(s.includes('search-empty') ? 'search-empty OK' : 'search-empty MISSING');
