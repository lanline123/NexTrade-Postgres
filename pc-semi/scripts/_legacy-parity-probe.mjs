// Read-only parity probe #2: effective page weight = page.tsx + the @/components/*Client
// modules it imports (this project puts the real page body in src/components/*Client.tsx).
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const oldPages = path.resolve(root, '../../../java-b2b2c-overseas-v5.8.61/view/pc/pages');
const appDir = path.resolve(root, 'app');
const srcDir = path.resolve(root, 'src');

function walk(dir, filter, out = []) {
  if (!fs.existsSync(dir)) return out;
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const full = path.join(dir, entry.name);
    if (entry.isDirectory()) walk(full, filter, out);
    else if (filter(entry.name)) out.push(full);
  }
  return out;
}

const norm = (route) =>
  route
    .toLowerCase()
    .replace(/\/index$/, '')
    .replace(/\/list$/, '')
    .replace(/\/info$/, '')
    .replace(/\/$/, '') || '/';

function nuxtRoute(file) {
  let r = path.relative(oldPages, file).replace(/\\/g, '/').replace(/\.vue$/, '');
  r = r.replace(/\/index$/, '').replace(/\[([^\]]+)\]/g, ':$1');
  return `/${r === 'index' ? '' : r}`;
}
function nextRoute(file) {
  let r = path.relative(appDir, file).replace(/\\/g, '/').replace(/\/?page\.tsx$/, '').replace(/\[([^\]]+)\]/g, ':$1');
  return `/${r}`.replace(/\/$/, '') || '/';
}

// resolve "@/components/Foo" style imports, one level deep, summing bytes
function resolveAlias(spec) {
  const rel = spec.replace(/^@\//, '');
  for (const base of [srcDir, root]) {
    for (const ext of ['.tsx', '.ts', '.jsx', '.js', '/index.tsx', '/index.ts']) {
      const p = path.join(base, rel + ext);
      if (fs.existsSync(p) && fs.statSync(p).isFile()) return p;
    }
  }
  return null;
}

function effective(file, seen = new Set()) {
  if (seen.has(file)) return 0;
  seen.add(file);
  let total = fs.statSync(file).size;
  const text = fs.readFileSync(file, 'utf8');
  const specs = [...text.matchAll(/from\s+['"](@\/[^'"]+|\.\.?\/[^'"]+)['"]/g)].map((m) => m[1]);
  for (const spec of specs) {
    if (spec.startsWith('@/') && !spec.includes('/api') && !spec.includes('/lib/')) {
      const p = resolveAlias(spec);
      if (p) total += effective(p, seen);
    }
  }
  return total;
}

const oldFiles = walk(oldPages, (n) => n.endsWith('.vue'));
const newFiles = walk(appDir, (n) => n === 'page.tsx');

const newByRoute = new Map();
for (const f of newFiles) {
  newByRoute.set(norm(nextRoute(f)), {
    file: path.relative(root, f),
    kb: +(effective(f) / 1024).toFixed(1),
  });
}

const rows = [];
for (const f of oldFiles) {
  const route = nuxtRoute(f);
  if (/\/src\//.test(route) || /\/components\//.test(route)) continue;
  if (/^\/activity\/pc/.test(route) || /\/index\.v1$/.test(route)) continue;
  const text = fs.readFileSync(f, 'utf8');
  const m = text.match(/<template>([\s\S]*?)<\/template>/);
  const tplLines = (m ? m[1] : '').split('\n').filter((l) => l.trim()).length;
  const nw = newByRoute.get(norm(route));
  rows.push({
    route,
    oldKb: +(fs.statSync(f).size / 1024).toFixed(1),
    tplLines,
    newFile: nw ? nw.file : '(none)',
    newKb: nw ? nw.kb : 0,
  });
}

const thin = rows.filter((r) => r.newKb < 3 && r.oldKb > 4).sort((a, b) => b.oldKb - a.oldKb);
if (process.argv.includes('--all')) {
  console.log(`legacy pages ${rows.length} / new pages ${newFiles.length}`);
  for (const r of rows.sort((a, b) => a.route.localeCompare(b.route))) {
    console.log(`  ${r.route.padEnd(40)} ${String(r.oldKb).padStart(6)}KB tpl~${String(r.tplLines).padStart(3)}  →  ${r.newFile}`);
  }
  process.exit(0);
}
console.log(`legacy pages ${rows.length} / new pages ${newFiles.length}`);
console.log(`\nTHIN after following 1 level of imports (legacy > 4KB, new effective < 3KB): ${thin.length}`);
for (const r of thin) {
  console.log(`  ${r.route.padEnd(40)} old ${String(r.oldKb).padStart(6)}KB  →  ${r.newFile} eff ${r.newKb}KB`);
}
const ratio = rows
  .filter((r) => r.oldKb > 8)
  .map((r) => ({ ...r, ratio: +(r.newKb / r.oldKb).toFixed(2) }))
  .sort((a, b) => a.ratio - b.ratio)
  .slice(0, 20);
console.log('\nLOWEST new/old byte ratio (legacy > 8KB):');
for (const r of ratio) console.log(`  ${r.ratio}  ${r.route.padEnd(40)} old ${String(r.oldKb).padStart(6)}KB → ${r.newFile} eff ${r.newKb}KB`);
