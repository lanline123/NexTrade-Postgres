#!/usr/bin/env node
/** 统计 out/ 里的 HTML 页数、metadata 文件与最新 mtime，用来核对 build 结果。 */
import fs from 'node:fs';
import path from 'node:path';

const ROOT = 'out';
const html = [];
const walk = (dir, depth) => {
  if (depth > 8) return;
  for (const e of fs.readdirSync(dir, { withFileTypes: true })) {
    const p = path.join(dir, e.name);
    if (e.isDirectory()) {
      if (e.name === '_next') continue;
      walk(p, depth + 1);
    } else if (e.name.endsWith('.html')) html.push(p);
  }
};
walk(ROOT, 0);

let newest = 0;
for (const f of html) newest = Math.max(newest, fs.statSync(f).mtimeMs);

console.log(`out/ HTML 页数: ${html.length}`);
console.log(`最新一页 mtime: ${new Date(newest).toISOString()}`);
console.log(`build id: ${fs.readFileSync(path.join(ROOT, 'index.html'), 'utf8').match(/\/_next\/static\/([^/"]+)\//)?.[1] ?? '(未识别)'}`);
for (const f of ['robots.txt', 'sitemap.xml', 'manifest.webmanifest', 'icon.png', 'apple-icon.png', 'favicon.ico']) {
  console.log(`  ${fs.existsSync(path.join(ROOT, f)) ? 'YES' : 'NO '}  ${f}`);
}
