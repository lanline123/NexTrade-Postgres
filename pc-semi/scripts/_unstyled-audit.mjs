// Read-only audit: which class names does the JSX use that no stylesheet ever styles?
// Unstyled markup = page renders as raw HTML, the classic parity miss.
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');

function walk(dir, filter, out = []) {
  for (const e of fs.readdirSync(dir, { withFileTypes: true })) {
    if (e.name === 'node_modules' || e.name === 'semi.css') continue;
    const full = path.join(dir, e.name);
    if (e.isDirectory()) {
      if (e.name.startsWith('.') || e.name === 'out') continue;
      walk(full, filter, out);
    } else if (filter(e.name)) out.push(full);
  }
  return out;
}

const tsx = [...walk(path.resolve(root, 'app'), (n) => n.endsWith('.tsx')), ...walk(path.resolve(root, 'src'), (n) => n.endsWith('.tsx'))];
const styles = [...walk(path.resolve(root, 'app'), (n) => n.endsWith('.scss') || n.endsWith('.css')), ...walk(path.resolve(root, 'src'), (n) => n.endsWith('.scss') || n.endsWith('.css'))];

const used = new Map(); // class -> Set(files)
for (const file of tsx) {
  const text = fs.readFileSync(file, 'utf8');
  const rel = path.relative(root, file);
  for (const m of text.matchAll(/className=(?:"([^"]*)"|\{`([^`]*)`\}|\{'([^']*)'\})/g)) {
    const raw = m[1] || m[2] || m[3] || '';
    for (const token of raw.split(/[\s]+/)) {
      if (!token || /[\{\}$():?,+&|]/.test(token)) continue;
      if (!used.has(token)) used.set(token, new Set());
      used.get(token).add(rel);
    }
  }
}

const defined = new Set();
for (const file of styles) {
  const text = fs.readFileSync(file, 'utf8');
  // strip comments so commented-out rules don't count as styled
  const clean = text.replace(/\/\*[\s\S]*?\*\//g, '').replace(/\/\/[^\n]*/g, '');
  for (const m of clean.matchAll(/\.([A-Za-z_][\w-]*)/g)) defined.add(m[1]);
  for (const m of clean.matchAll(/@extend\s+\.([A-Za-z_][\w-]*)/g)) defined.add(m[1]);
}

const missing = [...used.entries()]
  .filter(([cls]) => !defined.has(cls))
  .map(([cls, files]) => ({ cls, files: [...files] }))
  .sort((a, b) => b.files.length - a.files.length);

console.log(`classes used: ${used.size}   defined in stylesheets: ${defined.size}   UNSTYLED: ${missing.length}`);
console.log('\nTop unstyled classes (most files first):');
for (const m of missing.slice(0, 60)) console.log(`  ${m.cls.padEnd(34)} ${m.files.length} file(s)  ${m.files.slice(0, 3).join(', ')}`);
