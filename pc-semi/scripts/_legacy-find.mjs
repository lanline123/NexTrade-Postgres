import fs from 'node:fs';
import path from 'node:path';

const root = 'D:/nextrade/NexTrade/java-b2b2c-overseas-v5.8.61/view/pc';
const needle = process.argv[2] || 'cardPrice';
const maxHits = Number(process.argv[3] || 12);
if (!needle) throw new Error('needle 必填');

const skip = new Set(['node_modules', '.nuxt', '.output', 'dist', 'assets']);
let hits = 0;
const walk = (dir) => {
  if (hits >= maxHits) return;
  let entries = [];
  try {
    entries = fs.readdirSync(dir, { withFileTypes: true });
  } catch {
    return;
  }
  for (const entry of entries) {
    if (hits >= maxHits) return;
    if (entry.name.startsWith('.') || skip.has(entry.name)) continue;
    const full = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      walk(full);
      continue;
    }
    if (!/\.(ts|vue|js)$/.test(entry.name)) continue;
    const text = fs.readFileSync(full, 'utf8');
    if (!text.includes(needle)) continue;
    const lines = text.split('\n');
    lines.forEach((line, index) => {
      if (hits >= maxHits) return;
      if (!line.includes(needle)) return;
      hits += 1;
      console.log(`${path.relative(root, full)}:${index + 1}: ${line.trim().slice(0, 160)}`);
    });
  }
};
walk(root);
console.log(`-- ${hits} hit(s) for ${needle}`);
