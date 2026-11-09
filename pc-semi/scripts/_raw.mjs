import fs from 'node:fs';
const [file, from, to] = process.argv.slice(2);
const s = fs.readFileSync(file, 'utf8').split('\n');
const a = Math.max(0, (Number(from) || 1) - 1);
const b = to ? Number(to) : s.length;
console.log(s.slice(a, b).join('\n'));
