// 批量打印文本文件（≤ 240 行/文件，超出截断）。用法：node scripts/_cat.mjs f1 f2 ...
import fs from 'node:fs';

for (const f of process.argv.slice(2)) {
  console.log(`\n===== ${f}`);
  try {
    const s = fs.readFileSync(f, 'utf8').split('\n');
    const cap = Math.min(s.length, 240);
    for (let i = 0; i < cap; i += 1) console.log(`${String(i + 1).padStart(4)}| ${s[i]}`);
    if (s.length > cap) console.log(`      ... (${s.length - cap} more lines)`);
  } catch (err) {
    console.log(`   ! ${err.code}`);
  }
}
