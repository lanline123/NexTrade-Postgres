// 精确文本搜索（子串，不分词），递归 app/ 与 src/，跳过 node_modules/.next/out。
// 用法: node scripts/_find-text.mjs <needle> [maxHits]
import fs from 'node:fs';
import path from 'node:path';

const needle = (process.argv[2] || '').replace(/^["']|["']$/g, '');
const maxHits = Number(process.argv[3] || 40);
if (!needle) {
  console.error('usage: node scripts/_find-text.mjs <needle> [maxHits]');
  process.exit(1);
}
const EXTS = ['.tsx', '.ts', '.jsx', '.js', '.mjs', '.scss', '.css', '.json'];
const SKIP = new Set(['node_modules', '.next', '.next-dev', 'out', '.git', 'checks', 'capture', 'investigation.cany']);

let hits = 0;
function walk(dir) {
  if (hits >= maxHits) return;
  let entries;
  try {
    entries = fs.readdirSync(dir, { withFileTypes: true });
  } catch {
    return;
  }
  for (const entry of entries) {
    if (hits >= maxHits) return;
    const full = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      if (SKIP.has(entry.name)) continue;
      walk(full);
    } else if (EXTS.some((ext) => entry.name.endsWith(ext))) {
      let text;
      try {
        text = fs.readFileSync(full, 'utf8');
      } catch {
        continue;
      }
      if (!text.includes(needle)) continue;
      const lines = text.split(/\r?\n/);
      lines.forEach((line, i) => {
        if (hits < maxHits && line.includes(needle)) {
          hits += 1;
          console.log(`${full}:${i + 1}: ${line.trim().slice(0, 220)}`);
        }
      });
    }
  }
}

for (const root of ['app', 'src']) walk(root);
if (!hits) console.log(`-- no hit for ${JSON.stringify(needle)}`);
