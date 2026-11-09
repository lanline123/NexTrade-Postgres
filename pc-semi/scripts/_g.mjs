import fs from 'node:fs';
import path from 'node:path';

// 用法：node scripts/_g.mjs <needle> [relativePath] [extFilter]
const needle = process.argv[2];
const start = process.argv[3] || '.';
const ext = (process.argv[4] || '').toLowerCase();
const skip = new Set(['node_modules', '.git', '.next', 'out', '.tmp-r24', '.tmp-r27', '.tmp-r28', '.tmp-r29', '.tmp-r30']);
let hits = 0;
function walk(dir) {
  let entries = [];
  try {
    entries = fs.readdirSync(dir, { withFileTypes: true });
  } catch {
    return;
  }
  for (const entry of entries) {
    const full = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      if (skip.has(entry.name)) continue;
      walk(full);
      continue;
    }
    if (ext && !entry.name.toLowerCase().endsWith(ext)) continue;
    let text = '';
    try {
      const stat = fs.statSync(full);
      if (stat.size > 8_000_000) continue;
      text = fs.readFileSync(full, 'utf8');
    } catch {
      continue;
    }
    if (!text.includes(needle)) continue;
    const lines = text.split(/\r?\n/);
    lines.forEach((line, index) => {
      if (line.includes(needle)) {
        hits += 1;
        console.log(`${full}:${index + 1}: ${line.trim().slice(0, 160)}`);
      }
    });
  }
}
walk(start);
console.log(`-- ${hits} hit(s)`);
