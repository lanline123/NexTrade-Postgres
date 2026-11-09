// Read-only grep over the legacy Vue tree (styles, components) — the visual baseline.
// usage: node scripts/_legacy-grep.mjs "<regex>" [subdir] [maxHits]
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const legacy = path.resolve(root, '../../../java-b2b2c-overseas-v5.8.61/view/pc');
const unquote = (v) => (v || '').replace(/^["']|["']$/g, '');
const [pattern, sub = '', maxArg] = process.argv.slice(2).map(unquote);
const max = Number(maxArg || 40);
const re = new RegExp(pattern, 'i');
const exts = /\.(scss|css|less|vue|ts|js)$/;

function walk(d, out = []) {
  if (!fs.existsSync(d)) return out;
  for (const e of fs.readdirSync(d, { withFileTypes: true })) {
    if (e.name === 'node_modules' || e.name === '.nuxt') continue;
    const full = path.join(d, e.name);
    if (e.isDirectory()) walk(full, out);
    else if (exts.test(e.name)) out.push(full);
  }
  return out;
}

let hits = 0;
for (const file of walk(path.resolve(legacy, sub))) {
  const lines = fs.readFileSync(file, 'utf8').split('\n');
  for (let i = 0; i < lines.length && hits < max; i += 1) {
    if (re.test(lines[i])) {
      hits += 1;
      console.log(`${path.relative(legacy, file)}:${i + 1}: ${lines[i].trim().slice(0, 180)}`);
    }
  }
}
console.log(`-- ${hits} hit(s) in ${sub || 'view/pc'}`);
