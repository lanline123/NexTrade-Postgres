// Read-only window into the legacy Vue source tree (outside this project, so the
// workspace file reader can't reach it). Prints a page with line numbers, or greps it.
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const legacy = path.resolve(root, '../../../java-b2b2c-overseas-v5.8.61/view/pc');

const args = process.argv.slice(2);
const grepAt = args.indexOf('--grep');
let pattern = null;
if (grepAt !== -1) {
  pattern = args[grepAt + 1];
  args.splice(grepAt, 2);
}
const [rel, offsetArg, limitArg] = args;
if (!rel) {
  console.error('usage: node scripts/_legacy-read.mjs <path under view/pc> [offset] [limit] [--grep pattern]');
  process.exit(2);
}
const file = path.resolve(legacy, rel);
if (!file.startsWith(legacy) || !fs.existsSync(file)) {
  console.error(`not found: ${rel}`);
  process.exit(2);
}
const lines = fs.readFileSync(file, 'utf8').split('\n');
if (pattern) {
  const re = new RegExp(pattern, 'i');
  lines.forEach((l, i) => {
    if (re.test(l)) console.log(`${String(i + 1).padStart(5)} | ${l.trimEnd()}`);
  });
} else {
  const offset = Number(offsetArg || 1);
  const limit = Number(limitArg || 120);
  console.log(`# ${rel}  (${lines.length} lines)`);
  for (let i = offset - 1; i < Math.min(lines.length, offset - 1 + limit); i += 1) {
    console.log(`${String(i + 1).padStart(5)} | ${lines[i].replace(/\t/g, '  ')}`);
  }
}
