import fs from 'node:fs';
import path from 'node:path';

const needles = process.argv.slice(2).map((v) => v.replace(/^["']|["']$/g, ''));
if (!needles.length) {
  console.error('usage: node scripts/_find-any.mjs <a> <b> ...');
  process.exit(1);
}
const EXTS = ['.tsx', '.ts', '.jsx', '.js', '.mjs', '.scss', '.css'];
const SKIP = new Set(['node_modules', '.next', '.next-dev', 'out', '.git', 'checks', 'capture', 'public']);

function walk(dir, out = []) {
  let entries;
  try {
    entries = fs.readdirSync(dir, { withFileTypes: true });
  } catch {
    return out;
  }
  for (const entry of entries) {
    const full = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      if (!SKIP.has(entry.name)) walk(full, out);
    } else if (EXTS.some((ext) => entry.name.endsWith(ext))) {
      out.push(full);
    }
  }
  return out;
}

const files = [...walk('app'), ...walk('src')];
for (const needle of needles) {
  const hits = [];
  for (const file of files) {
    let text;
    try {
      text = fs.readFileSync(file, 'utf8');
    } catch {
      continue;
    }
    if (!text.includes(needle)) continue;
    text.split(/\r?\n/).forEach((line, i) => {
      if (line.includes(needle)) hits.push(`${file}:${i + 1}: ${line.trim().slice(0, 200)}`);
    });
  }
  console.log(`### ${needle} — ${hits.length} hit(s)`);
  for (const hit of hits.slice(0, 25)) console.log(hit);
}
