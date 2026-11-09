// 会员页 page.tsx 体量 + 是否仅转发：node scripts/_mp-size.mjs
import fs from 'node:fs';
import path from 'node:path';

const root = path.resolve(process.cwd(), 'app/member');
const files = [];
function walk(dir) {
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const full = path.join(dir, entry.name);
    if (entry.isDirectory()) walk(full);
    else if (entry.name === 'page.tsx') files.push(full);
  }
}
walk(root);

for (const file of files) {
  const text = fs.readFileSync(file, 'utf8');
  const rel = path.relative(process.cwd(), file).replace(/\\/g, '/');
  const isRedirect = /redirect\(|permanentRedirect\(/.test(text) && !/return \(/.test(text);
  const hasMp = /mp-[a-z0-9-]+/.test(text);
  const imports = [...text.matchAll(/from '([^']+)'/g)].map((m) => m[1]).filter((s) => s.includes('/'));
  console.log(
    String(text.length).padStart(6),
    isRedirect ? 'REDIRECT' : 'render  ',
    hasMp ? 'mp    ' : 'NO-MP ',
    rel,
    isRedirect ? '' : imports.map((i) => i.split('/').pop()).slice(0, 2).join('|'),
  );
}
