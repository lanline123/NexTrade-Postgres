import fs from 'node:fs';
const file = (process.argv[2] || 'app/decorate-source.scss').replace(/^["']+|["']+$/g, '');
const src = fs.readFileSync(file, 'utf8').replace(/\/\*[\s\S]*?\*\//g, ' ');
const lines = src.split(/\r?\n/);

const stack = [];          // 祖先选择器栈
const dangerous = [];      // 真的会污染全局的
const safe = [];           // 有 class 祖先兜住的
let blockLine = 0;
const blockNames = [];

for (let i = 0; i < lines.length; i += 1) {
  const raw = lines[i];
  const t = raw.trim();
  if (!t) continue;
  const m = t.match(/^(.*)\{\s*$/);
  if (m) {
    const sel = m[1].trim();
    if (sel && !sel.startsWith('@')) {
      const first = sel.split(',')[0].trim();
      const isBareTag = /^[a-z][a-z0-9-]*$/.test(first) || /^[a-z][a-z0-9-]*::?[a-z-]+$/.test(first) || /^[a-z][a-z0-9-]*:not\(/.test(first);
      if (isBareTag) {
        // 祖先里有没有 .class / #id / &
        const guarded = stack.some((s) => /^[.#&]/.test(s));
        const rec = { line: i + 1, sel: first, stack: stack.slice() };
        if (guarded) safe.push(rec);
        else dangerous.push(rec);
      }
      stack.push(sel);
    } else {
      stack.push('@');   // @media 之类不兜作用域
    }
  }
  const closes = (raw.match(/\}/g) || []).length;
  for (let k = 0; k < closes; k += 1) stack.pop();
}

console.log(`无 class 祖先兜住的裸标签（真全局）：${dangerous.length} 条`);
for (const d of dangerous) console.log(`  ${d.line}\t${d.sel}\t祖先=[${d.stack.join(' > ')}]`);
console.log(`\n有 class 祖先兜住的裸标签（等同旧版 scoped，安全）：${safe.length} 条`);
const byAncestor = {};
for (const s of safe) {
  const key = s.stack.find((x) => /^[.#&]/.test(x)) || '(无)';
  byAncestor[key] = (byAncestor[key] || 0) + 1;
}
for (const [k, v] of Object.entries(byAncestor)) console.log(`  ${v}\t${k}`);
