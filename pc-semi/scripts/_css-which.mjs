import fs from 'node:fs';
import path from 'node:path';

const dir = path.join(process.cwd(), 'out', '_next', 'static', 'css');
const needles = process.argv.slice(2);
for (const f of fs.readdirSync(dir).filter((x) => x.endsWith('.css'))) {
  const text = fs.readFileSync(path.join(dir, f), 'utf8');
  const hits = needles.filter((n) => text.includes(n));
  console.log(`${f}  ${hits.length ? hits.join(', ') : '-'}`);
}
