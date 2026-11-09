import fs from 'node:fs';
const file = process.argv[2] || 'out/index.html';
const h = fs.readFileSync(file, 'utf8');
const show = (label, re) => {
  const m = re.exec(h);
  console.log(label + ':', m ? m[0].slice(0, 200) : '(not found)');
};
show('manifest link', /<link[^>]+rel="manifest"[^>]*>/);
show('logo link', /<a[^>]+logo-link[^>]*>/);
show('search submit', /<button[^>]+search-submit[^>]*>/);
console.log('aria-labels:', (h.match(/aria-label="/g) || []).length);
console.log('empty-alt imgs:', (h.match(/alt=""/g) || []).length);
const manifestFile = 'out/manifest.webmanifest';
console.log('manifest file exists:', fs.existsSync(manifestFile), fs.existsSync(manifestFile) ? fs.statSync(manifestFile).size + ' bytes' : '');
console.log('robots:', fs.existsSync('out/robots.txt'), 'sitemap:', fs.existsSync('out/sitemap.xml'), 'icon:', fs.existsSync('out/icon.png'));
if (fs.existsSync(manifestFile)) console.log('manifest head:', fs.readFileSync(manifestFile, 'utf8').slice(0, 120));
