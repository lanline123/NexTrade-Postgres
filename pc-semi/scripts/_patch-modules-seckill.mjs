import fs from 'node:fs';
const file = 'src/decorate/components/modules.tsx';
let text = fs.readFileSync(file, 'utf8');
const log = [];
const apply = (from, to, label) => {
  if (!text.includes(from)) {
    log.push(`跳过（没找到）：${label}`);
    return;
  }
  text = text.replace(from, to);
  log.push(`已改：${label}`);
};

// 旧版 PcSeckillLegacy.vue:283-307（默认 flex 布局）：
//   .item-name.seckill-item__name > a.item-name-a
//   .item-action.seckill-item__price > .price-stack > .item-price > span.price.price-miaosha
//                                        + span.price.price-origin
//   .buy-container > .item-buy

// 1) 名称改成 a.item-name-a
apply(
  '                          {showName ? <div className="item-name seckill-item__name" style={{ textAlign: Number(module.textAlign) === 2 ? \'center\' : \'left\' }}>{product.productName}</div> : null}',
  `                          {showName ? (
                            <div className="item-name seckill-item__name" style={{ textAlign: Number(module.textAlign) === 2 ? 'center' : 'left' }}>
                              <Link
                                className="item-name-a"
                                href={product.seoLink || \`/product/\${product.productId ?? ''}\`}
                                target="_blank"
                              >
                                {product.productName}
                              </Link>
                            </div>
                          ) : null}`,
  'item-name > a.item-name-a',
);

// 2) .seckill-item__price → .item-action.seckill-item__price
apply(
  '<div className="seckill-item__price">',
  '<div className="item-action seckill-item__price">',
  'item-action',
);

// 3) 秒杀价包一层 .item-price；原价用 span（旧版是 FormatPrice 渲染的 span，
//    line-through 由 .seckill-item__price .price-origin 给，不用 <s>）
apply(
  '                                <span className="price price-miaosha">{formatPrice(product.seckillPrice ?? product.productPrice)}</span>\n                                {Number(product.productPrice || product.marketPrice) > 0 ? <s className="price price-origin">{formatPrice(product.productPrice ?? product.marketPrice)}</s> : null}',
  `                                <div className="item-price">
                                  <span className="price price-miaosha">{formatPrice(product.seckillPrice ?? product.productPrice)}</span>
                                </div>
                                {Number(product.productPrice || product.marketPrice) > 0 ? (
                                  <span className="price price-origin">{formatPrice(product.productPrice ?? product.marketPrice)}</span>
                                ) : null}`,
  'item-price 包裹 + 原价 span',
);

// 4) 加购容器 .buy → .item-buy（只改秒杀块里这一处：紧跟 buy-container 的 div）
apply(
  '                            <div className="buy-container">\n                              <div className="buy">',
  '                            <div className="buy-container">\n                              <div className="item-buy">',
  'item-buy',
);

fs.writeFileSync(file, text, 'utf8');
console.log(log.join('\n'));
