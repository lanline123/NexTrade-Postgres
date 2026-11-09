// 只打印旧版 .vue 的顶层 <template> 块（正确处理内部嵌套的 <template>），并报告 <style> 摘要。
// 用法：node scripts/_vue-tpl.mjs file.vue ...
import fs from 'node:fs';

function topTemplate(src) {
  const open = src.match(/<template(\s[^>]*)?>/);
  if (!open) return null;
  const start = open.index + open[0].length;
  let depth = 1;
  const re = /<template(\s[^>]*)?>|<\/template>/g;
  re.lastIndex = start;
  let m;
  while ((m = re.exec(src))) {
    if (m[0] === '</template>') {
      depth -= 1;
      if (depth === 0) return src.slice(start, m.index);
    } else {
      depth += 1;
    }
  }
  return src.slice(start);
}

for (const f of process.argv.slice(2)) {
  let s;
  try {
    s = fs.readFileSync(f, 'utf8');
  } catch (err) {
    console.log(`\n##### ${f}\n   ! ${err.code}`);
    continue;
  }
  console.log(`\n##### ${f}`);
  const tpl = topTemplate(s);
  if (tpl == null) {
    console.log('-- no <template>');
  } else {
    const lines = tpl.replace(/^\n/, '').replace(/\s+$/, '').split('\n');
    for (const l of lines) console.log(l);
    console.log(`-- template: ${lines.length} lines`);
  }
  const styles = [...s.matchAll(/<style([^>]*)>([\s\S]*?)<\/style>/g)];
  if (!styles.length) console.log('-- style: NONE');
  for (const m of styles) {
    const attrs = m[1].trim() || '(no attrs)';
    const body = m[2];
    console.log(`-- style[${attrs}]: ${body.length} B / ${body.split('\n').length} L`);
    for (const i of [...body.matchAll(/@import\s+[^;]+;/g)]) console.log(`     ${i[0].trim()}`);
    const lessish = [];
    if (/^\s*@[a-zA-Z-]+\s*:/m.test(body)) lessish.push('less-variable');
    if (/~["']/.test(body)) lessish.push('less-escape');
    if (/when\s*\(/.test(body)) lessish.push('less-guard');
    if (/\.\.\.\(/.test(body)) lessish.push('less-rest-args');
    if (lessish.length) console.log(`     !! ${lessish.join(', ')}`);
  }
}
