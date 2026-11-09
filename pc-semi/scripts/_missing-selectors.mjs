// 给一组旧版 .vue，抽出 <style> 里的顶层选择器，逐一检查是否已存在于 app/*.scss
// （= 已移植过，禁止重复搬运）。用法：node scripts/_missing-selectors.mjs file.vue ...
import fs from 'node:fs';
import path from 'node:path';

const APP = 'app';
const pool = fs
  .readdirSync(APP)
  .filter((f) => f.endsWith('.scss') || f.endsWith('.css'))
  .map((f) => fs.readFileSync(path.join(APP, f), 'utf8'))
  .join('\n');

function styleBody(src) {
  const blocks = [...src.matchAll(/<style[^>]*>([\s\S]*?)<\/style>/g)].map((m) => m[1]);
  return blocks.join('\n');
}

/** 极简解析：按深度抽出 depth===0 的选择器文本。 */
function topSelectors(css) {
  const out = new Set();
  let depth = 0;
  let buf = '';
  let quote = null;
  for (let i = 0; i < css.length; i += 1) {
    const c = css[i];
    if (quote) {
      buf += c;
      if (c === quote && css[i - 1] !== '\\') quote = null;
      continue;
    }
    if (c === '"' || c === "'") {
      quote = c;
      buf += c;
      continue;
    }
    if (c === '{') {
      const sel = buf.trim().replace(/\s+/g, ' ');
      buf = '';
      if (depth === 0 && sel && !sel.startsWith('//') && !sel.startsWith('/*')) {
        if (!sel.startsWith('@')) out.add(sel);
      }
      depth += 1;
      continue;
    }
    if (c === '}') {
      depth -= 1;
      buf = '';
      continue;
    }
    if (c === ';' && depth === 0) {
      buf = '';
      continue;
    }
    buf += c;
  }
  return [...out];
}

for (const f of process.argv.slice(2)) {
  let src;
  try {
    src = fs.readFileSync(f, 'utf8');
  } catch (err) {
    console.log(`! ${err.code}  ${f}`);
    continue;
  }
  const sels = topSelectors(styleBody(src));
  if (!sels.length) {
    console.log(`\n## ${path.basename(f)}  (no top-level selector)`);
    continue;
  }
  const missing = sels.filter((s) => {
    // 取选择器里最有辨识度的一段：最后一个类名/元素
    const last = s.split(/\s+|>/) .filter(Boolean).pop() || s;
    const core = (last.match(/\.[\w-]+/) || [last])[0];
    return !pool.includes(core);
  });
  console.log(`\n## ${path.basename(f)}  ${sels.length} top-level, ${missing.length} missing`);
  for (const m of missing.slice(0, 40)) console.log(`   - ${m}`);
}
