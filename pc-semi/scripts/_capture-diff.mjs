#!/usr/bin/env node
/**
 * 对比两个 capture.json（原版 capture/ vs 本次 checks/round-N）的关键数字：
 * 每个宽度的页面尺寸、区块数、标题数、图片数、字体、颜色，以及失败请求。
 *
 * 用法：node scripts/_capture-diff.mjs capture/capture.json checks/round-7/capture.json
 */
import fs from 'node:fs';

const [, , leftPath = 'capture/capture.json', rightPath = 'checks/round-7/capture.json'] = process.argv;

const read = (p) => JSON.parse(fs.readFileSync(p, 'utf8'));

function brief(label, p) {
  const c = read(p);
  console.log(`\n================ ${label}: ${p}`);
  console.log('keys:', Object.keys(c).join(', '));

  const a11y = c.a11y || {};
  console.log(`lang=${JSON.stringify(a11y.lang)} landmarks=${Object.keys(a11y.landmarks || {}).length} images_without_alt=${a11y.images_without_alt}`);

  const assets = c.assets || {};
  console.log(`fonts=${(assets.fonts || []).length} images=${(assets.images || []).length} backgroundImages=${(assets.background_images || []).length} requests=${(assets.requests || []).length}`);

  const health = c.health || {};
  const failed = health.failed_requests || [];
  console.log(`failed_requests=${failed.length}`);
  for (const f of failed.slice(0, 8)) console.log(`   - ${f.url || JSON.stringify(f).slice(0, 120)}`);

  const headings = c.headings || c.content?.headings || [];
  console.log(`headings=${headings.length}`, headings.slice(0, 6).map((h) => (typeof h === 'string' ? h : h.text)).join(' | '));

  const layouts = c.layouts || c.layout || [];
  const arr = Array.isArray(layouts) ? layouts : Object.values(layouts);
  console.log(`layout entries=${arr.length}`);
  for (const l of arr) {
    const width = l.width ?? l.viewportWidth ?? '?';
    const pageW = l.page?.width ?? l.pageWidth ?? '?';
    const pageH = l.page?.height ?? l.pageHeight ?? '?';
    const sections = l.sections || l.layout?.sections || [];
    const items = l.itemsPerRow ?? l.layout?.itemsPerRow;
    console.log(`  width=${width} page=${pageW}x${pageH} sections=${Array.isArray(sections) ? sections.length : '?'} itemsPerRow=${JSON.stringify(items)}`);
  }

  const tokens = c.tokens || c.design_tokens || {};
  const tkeys = Object.keys(tokens);
  if (tkeys.length) {
    console.log(`tokens: ${tkeys.join(', ')}`);
    for (const k of tkeys) {
      const v = tokens[k];
      if (Array.isArray(v)) console.log(`   ${k}(${v.length}): ${JSON.stringify(v.slice(0, 10))}`);
      else console.log(`   ${k}: ${JSON.stringify(v).slice(0, 200)}`);
    }
  }
}

brief('原版 capture/', leftPath);
brief('本次 check', rightPath);
