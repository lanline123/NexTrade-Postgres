import fs from 'node:fs';
const s = fs.readFileSync('.tmp-r27/audit-home.txt', 'utf8');
for (const block of s.split('### ').slice(1)) {
  const lines = block.split(/\r?\n/);
  const head = (lines[0] || '').trim();
  const body = lines.slice(1).filter((l) => l.trim() && !l.startsWith('--')).map((l) => l.trim()).join(' ');
  console.log(head);
  if (body) console.log('   ' + body);
}
