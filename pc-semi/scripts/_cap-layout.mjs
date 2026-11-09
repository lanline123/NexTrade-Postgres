import fs from 'node:fs';
const p = process.argv[2];
const j = JSON.parse(fs.readFileSync(p, 'utf8'));
const l = j.layouts || j.layout || [];
const list = Array.isArray(l) ? l : [l];
for (const lay of list) {
  console.log(`--- width ${lay.width} page_h=${lay.page_h} blocks=${(lay.blocks || []).length}`);
  if (lay.landmarks) console.log('landmarks:', JSON.stringify(lay.landmarks).slice(0, 900));
  for (const b of (lay.blocks || []).slice(0, 40)) {
    console.log(`  ${b.tag || b.role || '?'}\t${b.y}\t${b.h}\tp=${JSON.stringify(b.padding || '')}\tcls=${String(b.cls || b.class || '').slice(0, 80)}`);
  }
}
if (j.outline) {
  console.log('\n--- outline (top 60 by y)');
  const rows = [...j.outline].sort((a, b) => (a.y || 0) - (b.y || 0)).slice(0, 60);
  for (const r of rows) console.log(`  y=${r.y} h=${r.h} ${r.tag} "${String(r.text || r.name || '').slice(0, 40)}" ${String(r.cls || '').slice(0, 60)}`);
}
