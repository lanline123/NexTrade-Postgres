// 扫描旧版 .vue 的 <style> 块，报告会打断 SCSS 编译的 Less 专有语法。
// 用法：node scripts/_scan-less.mjs file.vue ...
import fs from 'node:fs';

const CHECKS = [
  ['less-var-decl', /^\s*@[a-zA-Z_][\w-]*\s*:\s*[^;{]+;/m],
  ['less-escape', /~["']/],
  ['less-guard', /\bwhen\s*\(/],
  ['less-dot-mixin-call', /^\s*\.[a-zA-Z_][\w-]*\([^)]*\)\s*;/m],
  ['less-nested-import', /@import\s*\(/],
  ['less-ampersand-plus', /&\+/],
  ['less-tilde-selector', /\b@\{/],
  ['less-operation', /:\s*[^;{}]*\b\d+(px|em|%)?\s*[*]\s*\d+/],
];

function styleBlocks(src) {
  return [...src.matchAll(/<style([^>]*)>([\s\S]*?)<\/style>/g)].map((m) => [m[1].trim(), m[2]]);
}

for (const f of process.argv.slice(2)) {
  let s;
  try {
    s = fs.readFileSync(f, 'utf8');
  } catch (err) {
    console.log(`! ${err.code}  ${f}`);
    continue;
  }
  const blocks = styleBlocks(s);
  if (!blocks.length) {
    console.log(`NO-STYLE            ${f}`);
    continue;
  }
  for (const [attrs, body] of blocks) {
    const hits = CHECKS.filter(([, re]) => re.test(body)).map(([name]) => name);
    const tag = hits.length ? `RISK(${hits.join(',')})` : 'ok';
    console.log(`${tag.padEnd(28)} ${String(body.length).padStart(6)}B  ${attrs || '-'}  ${f}`);
  }
}
