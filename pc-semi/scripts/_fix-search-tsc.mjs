#!/usr/bin/env node
/**
 * 一次性：修 tsc 报的 7 处错误。
 */
import fs from 'node:fs';

/** 删掉 [from, to] 1-based 闭区间的行（就地） */
function cut(file, from, to) {
  const lines = fs.readFileSync(file, 'utf8').split('\n');
  const removed = lines.splice(from - 1, to - from + 1);
  fs.writeFileSync(file, lines.join('\n'), 'utf8');
  console.log(`${file}: 删掉 ${removed.length} 行（${from}-${to}）`);
}

// 1) 搜索页：layoutCtx 块（旧版没有布局变体）整段删掉
cut('app/search/page.tsx', 528, 536);

// 2) 搜索页：装修开关类型 + 筛选面板入参
let s = fs.readFileSync('app/search/page.tsx', 'utf8');
s = s.replace(
  "  /* ---------- 旧版 SortFilter.vue / Filter.vue 需要的链接（服务端先算好） ---------- */",
  "  /* 旧版 SortFilter.vue 面板按「名称 → 值数组」取已选属性 */\n" +
    '  const selectedAttrsMap: Record<string, string[]> = {};\n' +
    '  for (const attr of selectedAttrs) selectedAttrsMap[attr.attrName] = attr.attrValue;\n\n' +
    '  /* ---------- 旧版 SortFilter.vue / Filter.vue 需要的链接（服务端先算好） ---------- */'
);
s = s.replace(
  "showSales={searchConfig?.showSales === true || searchConfig?.showSales === 1}",
  'showSales={Boolean(searchConfig?.showSales)}'
);
s = s.replace(
  "showShopTag={\n                            searchConfig?.showShopTag === true || searchConfig?.showShopTag === 1\n                          }",
  'showShopTag={Boolean(searchConfig?.showShopTag)}'
);
s = s.replace('attrs={esAttrResult}', 'attrs={esAttrs}');
s = s.replace('selectedAttrs={selectedAttrs}', 'selectedAttrs={selectedAttrsMap}');
fs.writeFileSync('app/search/page.tsx', s, 'utf8');
console.log(`app/search/page.tsx: ${s.length} 字节`);

// 3) ProductGrid：接口补 gridClassName（上次替换因换行符没命中）
let p = fs.readFileSync('src/components/ProductGrid.tsx', 'utf8');
if (!p.includes('gridClassName?: string;')) {
  p = p.replace(
    /(  \/\*\* 搜索装修 showShopTag：[^\n]*\n  showShopTag\?: boolean;\n)/,
    '$1' +
      '  /**\n' +
      '   * 额外挂到网格根节点上的旧版类名。\n' +
      '   * 旧版 pages/search/components/CatGoodsList.vue 的结构是\n' +
      '   * .goods-list__box > .goods-list-c > .wrap，.wrap 才是网格本体，\n' +
      '   * 所以搜索页把 wrap 传进来，让旧版的 grid-template-columns 生效。\n' +
      '   */\n' +
      '  gridClassName?: string;\n'
  );
  fs.writeFileSync('src/components/ProductGrid.tsx', p, 'utf8');
}
console.log(p.includes('gridClassName?: string;') ? 'ProductGridProps.gridClassName OK' : 'ProductGridProps.gridClassName MISSING');
