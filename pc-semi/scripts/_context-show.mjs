import fs from 'node:fs';
const file = process.argv[2] || 'out/index.html';
const needle = process.argv[3] || 'aria-label';
const html = fs.readFileSync(file, 'utf8');
let index = -1;
let count = 0;
while ((index = html.indexOf(needle, index + 1)) !== -1 && count < 6) {
  count += 1;
  console.log(`#${count} @${index}: …${html.slice(Math.max(0, index - 110), index + 150).replace(/\s+/g, ' ')}…`);
}
if (!count) console.log(`(no "${needle}" in ${file})`);
