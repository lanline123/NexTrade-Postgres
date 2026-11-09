import fs from 'node:fs';
const dir = 'node_modules/@icon-park/react';
if (!fs.existsSync(dir)) {
  console.log('not installed');
  process.exit(0);
}
console.log(fs.readdirSync(dir));
const pkg = JSON.parse(fs.readFileSync(`${dir}/package.json`, 'utf8'));
console.log('version', pkg.version, 'main', pkg.main, 'module', pkg.module, 'exports', JSON.stringify(pkg.exports).slice(0, 300));
for (const sub of ['es', 'lib']) {
  const p = `${dir}/${sub}`;
  if (fs.existsSync(p)) {
    const files = fs.readdirSync(p).filter((f) => f.endsWith('.js') && !f.endsWith('.map'));
    console.log(sub, 'js files', files.length, files.slice(0, 10));
    const all = files.find((f) => f === 'all.js');
    if (all) {
      const t = fs.readFileSync(`${p}/all.js`, 'utf8');
      console.log('all.js head', t.slice(0, 400));
    }
  }
}
