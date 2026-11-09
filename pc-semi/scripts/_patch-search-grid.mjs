#!/usr/bin/env node
/**
 * 一次性：
 *  1. 商品网格根节点同时挂 .mp-search-card —— 旧版 SearchProductCard.vue 的样式块
 *     全都挂在 .mp-search-card 下面，不挂这层的话卡片样式一条都不生效；
 *  2. 分页外面补旧版 CatGoodsList.vue 的 .pagination 包裹层。
 */
import fs from 'node:fs';

const file = 'app/search/page.tsx';
let s = fs.readFileSync(file, 'utf8');
const before = s.length;

s = s.replace('gridClassName="wrap"', 'gridClassName="wrap mp-search-card"');

// 分页：旧版是 .pagination > Pagination（Element Plus 根节点 .el-pagination）
s = s.replace(
  `                      <div className="el-page">
                        <nav className="el-pagination">`,
  `                      <div className="pagination">
                        <nav className="el-pagination">`
);

fs.writeFileSync(file, s, 'utf8');
console.log(`${file}: ${before} -> ${s.length} 字节`);
console.log(s.includes('wrap mp-search-card') ? 'mp-search-card OK' : 'mp-search-card MISSING');
console.log(s.includes('className="pagination"') ? '.pagination OK' : '.pagination MISSING');
