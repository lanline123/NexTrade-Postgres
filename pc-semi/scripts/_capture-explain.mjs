#!/usr/bin/env node
/**
 * 打印 capture.json 里 page / layouts / headings / design / outline 的真实结构。
 * 用法：node scripts/_capture-explain.mjs <capture.json> [key ...]
 */
import fs from 'node:fs';

const [, , p = 'capture/capture.json', ...keys] = process.argv;
const c = JSON.parse(fs.readFileSync(p, 'utf8'));

const show = (k) => {
  const v = c[k];
  if (v === undefined) {
    console.log(`\n--- ${k}: (missing)`);
    return;
  }
  const s = JSON.stringify(v, null, 2);
  console.log(`\n--- ${k} (${Array.isArray(v) ? `array ${v.length}` : typeof v})`);
  console.log(s.length > 4000 ? `${s.slice(0, 4000)}\n… trimmed ${s.length - 4000} chars` : s);
};

if (!keys.length) {
  for (const k of ['url', 'title', 'lang', 'meta', 'page', 'viewport', 'breakpoints', 'headings', 'text', 'design', 'outline', 'unreadable_stylesheets']) show(k);
} else {
  for (const k of keys) show(k);
}
