#!/usr/bin/env node
/** 一次性：ProductGrid 的 Props 接口补 gridClassName（兼容 CRLF） */
import fs from 'node:fs';
const file = 'src/components/ProductGrid.tsx';
let s = fs.readFileSync(file, 'utf8');
const doc =
  '  /**\r\n' +
  '   * 额外挂到网格根节点上的旧版类名。\r\n' +
  '   * 旧版 pages/search/components/CatGoodsList.vue 的结构是\r\n' +
  '   * .goods-list__box > .goods-list-c > .wrap，.wrap 才是网格本体\r\n' +
  '   * （grid-template-columns: repeat(var(--search-grid-columns, 5), minmax(0, 1fr))），\r\n' +
  '   * 所以搜索页把 wrap 传进来。\r\n' +
  '   */\r\n' +
  '  gridClassName?: string;\r\n';
if (!s.includes('gridClassName?: string;')) {
  s = s.replace(/(\n\s*showShopTag\?: boolean;\r?\n)/, `$1${doc}`);
  fs.writeFileSync(file, s, 'utf8');
}
console.log(s.includes('gridClassName?: string;') ? 'OK' : 'MISSING');
