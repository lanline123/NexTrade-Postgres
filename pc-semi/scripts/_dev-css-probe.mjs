#!/usr/bin/env node
/**
 * 抓 dev server 上某页面的所有 CSS 资源，统计若干选择器在“编译后的 CSS”里出现几次。
 * 用法：node scripts/_dev-css-probe.mjs <path> <needle…>
 */
const strip = (s) => (s || '').replace(/^["']+|["']+$/g, '');
const BASE = 'http://127.0.0.1:3004';
const pagePath = strip(process.argv[2]) || '/';
const needles = process.argv.slice(3).map(strip).filter(Boolean);

const html = await (await fetch(BASE + pagePath, { headers: { 'cache-control': 'no-cache' } })).text();
const hrefs = new Set();
const re = /<link[^>]+href="([^"]+\.css[^"]*)"[^>]*>/g;
let m;
while ((m = re.exec(html))) hrefs.add(m[1]);
// dev 模式下样式也可能走 <style> 内联
const inlineCss = [...html.matchAll(/<style[^>]*>([\s\S]*?)<\/style>/g)].map((x) => x[1]).join('\n');

let css = inlineCss;
const fetched = [];
for (const href of hrefs) {
  const url = href.startsWith('http') ? href : BASE + href;
  try {
    const text = await (await fetch(url, { headers: { 'cache-control': 'no-cache' } })).text();
    fetched.push(`${href} (${text.length}B)`);
    css += '\n' + text;
  } catch (e) {
    fetched.push(`${href} 失败: ${e.message}`);
  }
}
console.log(`页面 ${pagePath}: HTML ${html.length}B，内联样式 ${inlineCss.length}B`);
console.log('样式资源: ' + (fetched.length ? fetched.join(', ') : '（无外链）'));
console.log(`合计 CSS ${css.length}B`);
for (const n of needles) {
  console.log(`${String(css.split(n).length - 1).padStart(4)}  ${n}`);
}
