#!/usr/bin/env node
/**
 * 抓开发服务器上的一页，检查旧版类名是否真的渲染出来了。
 * 用法：node scripts/_dev-probe.mjs <path> className... 
 *       node scripts/_dev-probe.mjs "/search?keyword=电池" search-page sort-filter-wrap goods-list-c
 */
const base = 'http://127.0.0.1:3004';
const args = process.argv.slice(2).map((a) => a.replace(/^["']+|["']+$/g, ''));
const path = args[0] || '/';
const needles = args.slice(1);
const url = base + path;

const res = await fetch(url, { headers: { accept: 'text/html' } });
const html = await res.text();
console.log(`GET ${url} -> ${res.status}  ${html.length} bytes`);

const count = (re) => (html.match(re) || []).length;
console.log(`元素数 ~ ${count(/<[a-zA-Z][^>]*>/g)}`);
console.log(`class 属性 ${count(/class="/g)}`);
console.log(`<img> ${count(/<img\b/g)}`);

for (const needle of needles) {
  const n = html.split(needle).length - 1;
  console.log(`  ${needle.padEnd(34)} ${n} 处`);
}

const err = html.match(/Application error|Unhandled Runtime|__next_error__|digest&quot;/);
console.log(err ? `页面报错标记：${err[0]}` : '无错误标记');
