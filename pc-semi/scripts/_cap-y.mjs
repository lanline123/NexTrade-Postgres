#!/usr/bin/env node
/**
 * 列出 capture.json outline 里 y 落在指定区间内的条目（tag/class/text/box）。
 * 用法：node scripts/_cap-y.mjs .tmp-r29/cap-reco/capture.json 6500 7050
 */
import fs from 'node:fs';

const file = process.argv[2];
const from = Number(process.argv[3] || 0);
const to = Number(process.argv[4] || 1e9);
const cap = JSON.parse(fs.readFileSync(file, 'utf8'));

const r2 = (v) => Math.round(v);
const items = [];
const walk = (node) => {
  if (!node || typeof node !== 'object') return;
  if (Array.isArray(node)) {
    node.forEach(walk);
    return;
  }
  const box = node.box || node.rect || null;
  const y = box ? Number(box.y ?? box.top ?? NaN) : NaN;
  if (Number.isFinite(y) && y >= from && y <= to) {
    items.push({
      tag: node.tag || node.name || node.role || '?',
      cls: node.class || node.className || node.classes || '',
      text: String(node.text ?? node.label ?? '').slice(0, 40),
      x: r2(Number(box.x ?? box.left ?? 0)),
      y: r2(y),
      w: r2(Number(box.w ?? box.width ?? 0)),
      h: r2(Number(box.h ?? box.height ?? 0)),
    });
  }
  Object.values(node).forEach((value) => {
    if (value && typeof value === 'object') walk(value);
  });
};
walk(cap.outline ?? cap.outlines ?? cap);
console.log(`outline items in y ${from}..${to}: ${items.length}`);
items.sort((a, b) => a.y - b.y || a.x - b.x);
for (const item of items) {
  console.log(`  y=${item.y} x=${item.x} ${item.w}x${item.h} <${item.tag}> ${Array.isArray(item.cls) ? item.cls.join('.') : item.cls} ${item.text}`);
}
