// 生成的 .mp-<slug> 包裹类，哪些在源码里真的用上了：node scripts/_mp-usage.mjs
import fs from 'node:fs';
import path from 'node:path';

const scss = fs.readFileSync(path.resolve(process.cwd(), 'app/member-pages.scss'), 'utf8');
// 生成器用 `.mp-<slug> {` 作为每页的包裹根
const wrappers = [...new Set([...scss.matchAll(/^\.mp-([a-z0-9-]+)\s*\{/gm)].map((m) => m[1]))].sort();
console.log('generated wrapper roots:', wrappers.length);

const sources = [];
function walk(dir) {
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    if (['node_modules', '.next', 'out', '.git'].includes(entry.name)) continue;
    const full = path.join(dir, entry.name);
    if (entry.isDirectory()) walk(full);
    else if (/\.(tsx|ts)$/.test(entry.name)) sources.push(full);
  }
}
walk(path.resolve(process.cwd(), 'app'));
walk(path.resolve(process.cwd(), 'src'));

const blob = sources.map((f) => fs.readFileSync(f, 'utf8')).join('\n');
const used = [];
const unused = [];
for (const w of wrappers) {
  if (blob.includes(`mp-${w}`)) used.push(w);
  else unused.push(w);
}
console.log('used in source:', used.length, used.join(', '));
console.log('NOT used:', unused.length);
console.log(unused.join(', '));
