import fs from 'node:fs';
const p = process.argv[2] || '.tmp-r28/cap-preview2/capture.json';
const j = JSON.parse(fs.readFileSync(p, 'utf8'));
console.log('top keys:', Object.keys(j).join(', '));
for (const [k, v] of Object.entries(j)) {
  if (typeof v === 'string') console.log(`  ${k}: string ${v.length}B  head=${JSON.stringify(v.slice(0, 80))}`);
  else if (Array.isArray(v)) console.log(`  ${k}: array ${v.length}`);
  else if (v && typeof v === 'object') console.log(`  ${k}: object {${Object.keys(v).join(',')}}`);
}
