import fs from 'node:fs';
const p = process.argv[2] || '.tmp-r28/cap-preview2/capture.json';
const cls = process.argv[3] || 'item';
const j = JSON.parse(fs.readFileSync(p, 'utf8'));
const out = (j.outline || []).filter((o) => typeof o.class === 'string' && o.class.split(/\s+/).includes(cls));
console.log(`${cls}: ${out.length}`);
for (const o of out) {
  const b = o.box || {};
  console.log(`  y=${b.y} x=${b.x} ${b.w}x${b.h} tag=${o.tag} text=${JSON.stringify((o.text || '').slice(0, 24))}`);
}
