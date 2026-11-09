import fs from 'node:fs';
const dirs = fs.readdirSync('app', { withFileTypes: true });
console.log(
  'app top-level:',
  dirs
    .map((d) => d.name + (d.isDirectory() ? '/' : ''))
    .filter((n) => /manifest|icon|apple|robots|sitemap/i.test(n))
    .join(', ') || '(none)',
);
const j = JSON.parse(fs.readFileSync('checks/round-2/capture.json', 'utf8'));
console.log('capture.json keys:', Object.keys(j).join(', '));
const a11y = j.a11y || j.accessibility || j.a11ySummary || null;
console.log('a11y:', JSON.stringify(a11y).slice(0, 1500));
const health = j.health || j.pageHealth || null;
console.log('health:', JSON.stringify(health).slice(0, 1500));
