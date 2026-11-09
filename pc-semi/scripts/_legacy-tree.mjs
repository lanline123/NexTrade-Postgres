// 递归列出旧版 view/pc/pages 下指定子路径的文件（含字节数）。
// 用法：node scripts/_legacy-tree.mjs shop join joining exchange giftcard chat member/login.vue
import fs from 'node:fs';
import path from 'node:path';

const ROOT = 'D:/nextrade/NexTrade/java-b2b2c-overseas-v5.8.61/view/pc/pages';

function walk(p, out) {
  let ents;
  try {
    ents = fs.readdirSync(p, { withFileTypes: true });
  } catch {
    return;
  }
  for (const e of ents) {
    const full = path.join(p, e.name);
    if (e.isDirectory()) walk(full, out);
    else out.push([full, fs.statSync(full).size]);
  }
}

for (const rel of process.argv.slice(2)) {
  const start = path.join(ROOT, rel);
  const out = [];
  if (fs.existsSync(start) && fs.statSync(start).isFile()) {
    out.push([start, fs.statSync(start).size]);
  } else {
    walk(start, out);
  }
  console.log(`== pages/${rel}  (${out.length} file(s))`);
  for (const [full, size] of out.sort()) {
    console.log(`   ${String(size).padStart(7)}  pages/${path.relative(ROOT, full).split(path.sep).join('/')}`);
  }
}
