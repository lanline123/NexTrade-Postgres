#!/usr/bin/env node
/**
 * 取一条商品列表样本，打印第一条记录的键名与图片相关字段。
 * 用法：node scripts/_product-shape.mjs [url]
 */
const url = process.argv[2] || 'http://127.0.0.1:3004/api/product/product/list?page=1&size=2&productSelectType=0';
const response = await fetch(url);
const text = await response.text();
console.log(`GET ${url} -> ${response.status} ${text.length}B`);
let json = null;
try {
  json = JSON.parse(text);
} catch {
  console.log(text.slice(0, 300));
  process.exit(0);
}
const data = json?.data;
const records = Array.isArray(data) ? data : data?.records || [];
console.log(`records: ${records.length}`);
const first = records[0];
if (first) {
  const keys = Object.keys(first);
  console.log(`keys(${keys.length}): ${keys.join(', ')}`);
  for (const key of keys) {
    if (/img|pic|image/i.test(key)) console.log(`  ${key} = ${JSON.stringify(first[key]).slice(0, 120)}`);
  }
  console.log(`  productPrice = ${JSON.stringify(first.productPrice)} marketPrice = ${JSON.stringify(first.marketPrice)}`);
  console.log(`  productId = ${JSON.stringify(first.productId)} productSn = ${JSON.stringify(first.productSn)}`);
}
