import fs from 'node:fs';

// 1) ProductGrid 的函数签名（单行形式）
{
  const file = 'src/components/ProductGrid.tsx';
  let text = fs.readFileSync(file, 'utf8');
  const needle = 'export default function ProductGrid({ products, showSales = false, showShopTag = false, gridClassName }: ProductGridProps) {';
  const replacement = `export default function ProductGrid({
  products,
  showSales = false,
  showShopTag = false,
  gridClassName,
  fixedNameTwoLines = false,
}: ProductGridProps) {`;
  if (!text.includes(needle)) {
    console.error('ProductGrid 签名没找到');
    process.exit(1);
  }
  text = text.replace(needle, replacement);
  fs.writeFileSync(file, text, 'utf8');
  console.log('ProductGrid 签名已更新');
}

// 2) ProductCardItem 加 activityInfo
{
  const file = 'src/components/product/ProductCard.tsx';
  let text = fs.readFileSync(file, 'utf8');
  const needle = '  presaleInfo?: { saleMode?: number | string };';
  if (!text.includes(needle)) {
    console.error('ProductCardItem presaleInfo 没找到');
    process.exit(1);
  }
  text = text.replace(
    needle,
    `${needle}\n  /** 旧版活动信息：[{ type }]，搜索页用它渲染 .activity-tags */\n  activityInfo?: Array<{ type?: number | string }>;`,
  );
  fs.writeFileSync(file, text, 'utf8');
  console.log('ProductCardItem 已加 activityInfo');
}
