// 在一行 HTML 里找某个片段并在它前后开窗口：node scripts/_snippet.mjs <file> <needle> [before] [after]
import fs from 'node:fs';

const file = process.argv[2];
const needle = process.argv[3];
const before = Number(process.argv[4] || 400);
const after = Number(process.argv[5] || 600);
const text = fs.readFileSync(file, 'utf8');
let from = 0;
let count = 0;
while (count < 6) {
  const at = text.indexOf(needle, from);
  if (at < 0) break;
  count += 1;
  const start = Math.max(0, at - before);
  console.log(`--- hit ${count} @${at}`);
  console.log(text.slice(start, at + after));
  from = at + needle.length;
}
if (!count) console.log('no hit');
