// 从 dev server 取页面 HTML → 列出 CSS 资源 → 在其中查找一段文本，打印命中处的上下文。
// 用法: node scripts/_dev-css-check.mjs <path> <needle> [r] [context]
const strip = (s) => String(s || '').replace(/^["']|["']$/g, '');
const base = 'http://127.0.0.1:3004';
const p = strip(process.argv[2]) || '/';
const needle = strip(process.argv[3]) || '.pc-container';
const r = Number(strip(process.argv[4]) || 40);
const ctx = Number(strip(process.argv[5]) || 260);
const html = await (await fetch(base + p)).text();
const hrefs = new Set();
for (const m of html.matchAll(/href="([^"]+\.css[^"]*)"/g)) hrefs.add(m[1]);
for (const m of html.matchAll(/"(\/_next\/static\/css\/[^"]+)"/g)) hrefs.add(m[1]);
console.log(`HTML ${html.length} 字节，CSS 资源 ${hrefs.size} 个`);
let hits = 0;
for (const h of hrefs) {
  const url = h.startsWith('http') ? h : base + h;
  const css = await (await fetch(url)).text();
  let i = -1;
  while ((i = css.indexOf(needle, i + 1)) !== -1) {
    hits += 1;
    if (hits > r) break;
    console.log(`[${h}] @${i}`);
    console.log('  ' + css.slice(Math.max(0, i - ctx), i + ctx).replace(/\s+/g, ' '));
  }
  if (hits > r) break;
}
console.log(`共 ${hits} 处（上限 ${r}）`);
