#!/usr/bin/env node
/** 对比两个 capture.json 的 page/layouts/links/outline 规模。用法：node scripts/_cap-size.mjs a.json b.json */
import fs from 'node:fs';

for (const file of process.argv.slice(2)) {
  const cap = JSON.parse(fs.readFileSync(file, 'utf8'));
  const layouts = (cap.layouts ?? []).map((l) => ({
    w: l.width,
    h: l.page_h,
    blocks: Array.isArray(l.blocks) ? l.blocks.length : undefined,
    items: l.itemsPerRow,
  }));
  console.log(`${file}`);
  console.log(`  page=${JSON.stringify(cap.page)} layouts=${JSON.stringify(layouts)}`);
  console.log(`  outline=${(cap.outline ?? []).length} links=${(cap.links ?? []).length} images=${(cap.assets?.images ?? cap.images ?? []).length} text=${String(cap.text ?? '').length}`);
}
