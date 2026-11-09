import fs from 'node:fs';
const file = process.argv[2] || 'out/index.html';
const h = fs.readFileSync(file, 'utf8');
console.log('file', file, 'bytes', h.length, 'label', 'static snapshot sanity check');
console.log('title:', /<title>([^<]*)<\/title>/.exec(h)?.[1]);
const css = [...h.matchAll(/href="(\/_next\/static\/css\/[^"]+)"/g)].map((m) => m[1]);
console.log('css links:', css.length, css[0] || '(none)');
if (css[0]) console.log('css file exists:', fs.existsSync(`out${css[0]}`));
const js = [...h.matchAll(/src="(\/_next\/static\/chunks\/[^"]+)"/g)].map((m) => m[1]);
console.log('js chunks:', js.length, js[0] || '(none)');
if (js[0]) console.log('js file exists:', fs.existsSync(`out${js[0]}`));
console.log('leftover /_next/image:', (h.match(/\/_next\/image/g) || []).length);
for (const needle of ['pc-container', 'PcHeader', 'footer', 'pc-header']) {
  console.log(`contains ${needle}:`, h.includes(needle));
}
const total = fs.readdirSync('out').length;
console.log('out top-level entries:', total);
