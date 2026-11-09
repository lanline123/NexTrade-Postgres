import fs from 'node:fs';
const dir = 'D:/nextrade/NexTrade/java-b2b2c-overseas-v5.8.61/view/pc/components/home';
for (const f of fs.readdirSync(dir).filter((x) => x.endsWith('.vue')).sort()) {
  const s = fs.readFileSync(dir + '/' + f, 'utf8');
  const tpl = (s.match(/<template>([\s\S]*)<\/template>/) || [])[1] || '';
  const sty = (s.match(/<style[^>]*>([\s\S]*)<\/style>/) || [])[1] || '';
  console.log(
    f.padEnd(24),
    'total', String(s.length).padStart(6),
    'tpl', String(tpl.split('\n').length).padStart(4),
    'style', String(sty.split('\n').length).padStart(4),
  );
}
