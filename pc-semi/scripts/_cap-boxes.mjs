#!/usr/bin/env node
/**
 * 从 web_site_capture 的 capture.json 里挑出元素外框，按宽度和 y 区间过滤。
 * 用法：node scripts/_cap-boxes.mjs <capture.json> [minW] [maxW] [yMin] [yMax]
 * 输出：y 升序，x y w h tag "text" 前 60 字
 */
import fs from 'node:fs';

const file = process.argv[2];
const minW = Number(process.argv[3] || 0);
const maxW = Number(process.argv[4] || 1e9);
const yMin = Number(process.argv[5] || -1e9);
const yMax = Number(process.argv[6] || 1e9);
const data = JSON.parse(fs.readFileSync(file, 'utf8'));

const seen = new Set();
const rows = [];
const walk = (node) => {
  if (!node || typeof node !== 'object') return;
  if (Array.isArray(node)) {
    node.forEach(walk);
    return;
  }
  if (typeof node.w === 'number' && typeof node.h === 'number') {
    if (node.w >= minW && node.w <= maxW && node.y >= yMin && node.y <= yMax) {
      const key = `${node.x},${node.y},${node.w},${node.h},${node.tag || ''},${(node.text || '').slice(0, 40)}`;
      if (!seen.has(key)) {
        seen.add(key);
        rows.push({
          y: Math.round(node.y),
          x: Math.round(node.x),
          w: Math.round(node.w),
          h: Math.round(node.h),
          tag: node.tag || node.role || node.kind || '?',
          cls: (node.class || node.className || node.classes || '') + '',
          text: (node.text || node.name || node.label || '').replace(/\s+/g, ' ').slice(0, 60),
        });
      }
    }
  }
  for (const key of Object.keys(node)) {
    const v = node[key];
    if (v && typeof v === 'object') walk(v);
  }
};
walk(data);

rows.sort((a, b) => a.y - b.y || a.x - b.x);
console.log(`${rows.length} box(es) width ${minW}..${maxW}, y ${yMin}..${yMax}`);
for (const r of rows.slice(0, 120)) {
  console.log(`y${String(r.y).padStart(5)} x${String(r.x).padStart(4)} ${String(r.w).padStart(4)}x${String(r.h).padStart(4)}  ${r.tag.padEnd(12)} ${r.cls.slice(0, 34).padEnd(34)} ${r.text}`);
}
