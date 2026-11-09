#!/usr/bin/env node
// 列目录：node scripts/_ls.mjs <dir>
import { readdirSync, statSync } from 'node:fs';
import { join } from 'node:path';

const dir = (process.argv[2] || '.').replace(/^["']+|["']+$/g, '');
for (const name of readdirSync(dir).sort()) {
  const p = join(dir, name);
  let isDir = false;
  try { isDir = statSync(p).isDirectory(); } catch { /* ignore */ }
  console.log(isDir ? `${name}/` : name);
}
