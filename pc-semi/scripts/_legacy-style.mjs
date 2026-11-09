import fs from 'node:fs';
import path from 'node:path';

const ROOT = 'D:/nextrade/NexTrade/java-b2b2c-overseas-v5.8.61/view/pc';
const files = process.argv.slice(2);
for (const rel of files) {
  const p = path.join(ROOT, rel);
  if (!fs.existsSync(p)) {
    console.log(`\n===== ${rel} -> NOT FOUND`);
    continue;
  }
  const src = fs.readFileSync(p, 'utf8').split(/\r?\n/);
  const start = src.findIndex((l) => /^<style/.test(l.trim()));
  console.log(`\n===== ${rel}  (${src.length} lines, style at ${start + 1})`);
  if (start < 0) continue;
  console.log(src.slice(start).join('\n'));
}
