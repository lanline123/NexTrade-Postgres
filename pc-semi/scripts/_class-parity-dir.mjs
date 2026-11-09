/**
 * 目录版类名对照：把旧版某目录下所有 .vue 的 class 收集起来，和 TSX 里的 class 比，报缺失。
 * 用法: node scripts/_class-parity-dir.mjs <legacyDir> <tsx...>
 */
import fs from 'node:fs';
import path from 'node:path';

const argv = process.argv.slice(2).map((a) => a.replace(/^["']+|["']+$/g, ''));
const dir = argv[0];
const tsxFiles = argv.slice(1);
if (!dir || !tsxFiles.length) {
  console.error('用法: node scripts/_class-parity-dir.mjs <legacyDir> <tsx...>');
  process.exit(1);
}

function walk(d, out = []) {
  for (const e of fs.readdirSync(d, { withFileTypes: true })) {
    const p = path.join(d, e.name);
    if (e.isDirectory()) walk(p, out);
    else if (e.name.endsWith('.vue')) out.push(p);
  }
  return out;
}

const STOP = new Set(['flex', 'align-center', 'justify-between', 'text-ellipsis', 'ellipsis', 'red', 'blue', 'active', 'ml10', 'mt10', 'mb10', 'hand-pointer', 'flex_cc', 'flex_csb', 'flex_ce', 'align-start', 'align-end', 'flex-column', 'relative', 'border', 'item', 'title', 'name', 'info', 'list', 'type', 'desc', 'top', 'bottom']);

const legacy = new Map(); // class -> Set(files)
for (const f of walk(dir)) {
  const src = fs.readFileSync(f, 'utf8');
  const m = src.match(/<template>([\s\S]*?)<\/template>\s*(<script|$)/);
  const tpl = m ? m[1] : src;
  const re = /class="([^"]*)"/g;
  let mm;
  while ((mm = re.exec(tpl))) {
    const cleaned = mm[1].replace(/\{\{[\s\S]*?\}\}/g, ' ').replace(/[{}:]/g, ' ');
    for (const c of cleaned.split(/\s+/)) {
      if (!c || c.includes('$') || c.length > 40) continue;
      if (!legacy.has(c)) legacy.set(c, new Set());
      legacy.get(c).add(path.basename(f));
    }
  }
}

const tsx = tsxFiles.map((f) => fs.readFileSync(f, 'utf8')).join('\n');
const present = new Set();
for (const m of tsx.matchAll(/className=(?:"([^"]*)"|\{`([^`]*)`\}|\{'([^']*)'\})/g)) {
  const raw = m[1] || m[2] || m[3] || '';
  for (const c of raw.replace(/\$\{[\s\S]*?\}/g, ' ').split(/\s+/)) {
    if (c) present.add(c);
  }
}

const missing = [];
for (const [c, files] of legacy) {
  if (present.has(c)) continue;
  missing.push([c, [...files].join(',')]);
}
console.log(`旧版类名 ${legacy.size} 个，TSX 里已有 ${present.size} 个 token，缺失 ${missing.length} 个`);
const real = missing.filter(([c]) => !STOP.has(c));
console.log(`-- 去掉通用工具类后缺 ${real.length} 个:`);
for (const [c, f] of real) console.log(`   ${c}   <- ${f}`);
