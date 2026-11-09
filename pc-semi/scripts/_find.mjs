#!/usr/bin/env node
// 递归找文件名：node scripts/_find.mjs <root> <namePart> [maxDepth=8]
import { readdirSync, statSync } from 'node:fs';
import { join } from 'node:path';

const root = (process.argv[2] || '.').replace(/^["']+|["']+$/g, '');
const needle = (process.argv[3] || '').toLowerCase();
const maxDepth = Number(process.argv[4] || 8);
let hits = 0;

function walk(dir, depth) {
  if (depth > maxDepth) return;
  let names;
  try { names = readdirSync(dir); } catch { return; }
  for (const name of names) {
    if (name === 'node_modules' || name === '.git' || name === '.output') continue;
    const p = join(dir, name);
    let st;
    try { st = statSync(p); } catch { continue; }
    if (st.isDirectory()) walk(p, depth + 1);
    else if (name.toLowerCase().includes(needle)) { console.log(p); hits += 1; }
  }
}

walk(root, 0);
if (!hits) console.log(`-- 0 hit for ${needle}`);
