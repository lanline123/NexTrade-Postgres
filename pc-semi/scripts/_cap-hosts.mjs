#!/usr/bin/env node
/** 统计一份 capture.json 里所有图片 src 的域名分布，并列出失败的请求。 */
import fs from 'node:fs';

const file = process.argv[2];
const raw = JSON.parse(fs.readFileSync(file, 'utf8'));

function walk(node, out) {
  if (!node || typeof node !== 'object') return out;
  if (Array.isArray(node)) {
    for (const item of node) walk(item, out);
    return out;
  }
  for (const [key, value] of Object.entries(node)) {
    if (typeof value === 'string' && /^(https?:)?\/\//.test(value) && /\.(png|jpe?g|webp|gif|svg)(\?|$)/i.test(value)) {
      out.push(value);
    }
    walk(value, out);
  }
  return out;
}

const urls = [...new Set(walk(raw, []))];
const byHost = new Map();
for (const url of urls) {
  const host = url.replace(/^https?:\/\//, '').split('/')[0];
  byHost.set(host, (byHost.get(host) || 0) + 1);
}
console.log(`${file}: ${urls.length} unique image url(s)`);
for (const [host, count] of [...byHost].sort((a, b) => b[1] - a[1])) {
  console.log(`  ${host}  ${count}`);
}

const failed = raw?.health?.failed_requests || [];
console.log(`\nfailed_requests: ${failed.length}`);
for (const item of failed.slice(0, 20)) {
  console.log(`  ${typeof item === 'string' ? item : JSON.stringify(item).slice(0, 200)}`);
}

for (const layout of raw?.layouts || []) {
  console.log(`layout width=${layout.width ?? '?'} page_h=${layout.page_h ?? '?'} blocks=${(layout.blocks || []).length}`);
}
console.log(`page: ${JSON.stringify(raw?.page)} title=${JSON.stringify(raw?.title)} lang=${JSON.stringify(raw?.lang)} serverRendered=${JSON.stringify(raw?.serverRendered)}`);
console.log(`headings=${(raw?.headings || []).length} images=${(raw?.images || []).length} links=${(raw?.links || []).length} text_len=${String(raw?.text || '').length}`);
