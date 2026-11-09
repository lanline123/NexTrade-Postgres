import fs from 'node:fs';
const file = 'src/components/ProductGrid.tsx';
let text = fs.readFileSync(file, 'utf8');

const before = text;
// 1) 接口里加 fixedNameTwoLines
text = text.replace(
  /(  gridClassName\?: string;\r?\n)/,
  '$1  /** 旧版 SearchProductCard.vue 的 fixed-name-two-lines：搜索页固定两行名称。 */\r\n  fixedNameTwoLines?: boolean;\r\n',
);

// 2) 解构默认值 + 传给卡片
text = text.replace(
  /export default function ProductGrid\(\{\s*products,\s*showSales = false,\s*showShopTag = false,\s*gridClassName,\s*\}\) \{/,
  `export default function ProductGrid({
  products,
  showSales = false,
  showShopTag = false,
  gridClassName,
  fixedNameTwoLines = false,
}: ProductGridProps) {`,
);

text = text.replace(
  /          showShopTag=\{showShopTag\}\r?\n/,
  '          showShopTag={showShopTag}\r\n          fixedNameTwoLines={fixedNameTwoLines}\r\n          activityInfo={(product as ProductCardItem).activityInfo}\r\n',
);

if (text === before) {
  console.error('没有改动，正则没命中');
  process.exit(1);
}
fs.writeFileSync(file, text, 'utf8');
console.log('已更新 ProductGrid.tsx');
