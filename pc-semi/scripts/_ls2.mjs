// 列出多个目录（一次调用），只列文件名。用法：node scripts/_ls2.mjs dir1 dir2 ...
import fs from 'node:fs';
import path from 'node:path';

for (const dir of process.argv.slice(2)) {
  console.log(`== ${dir}`);
  try {
    const ents = fs.readdirSync(dir, { withFileTypes: true });
    for (const e of ents) console.log(`   ${e.isDirectory() ? '[d]' : '   '} ${e.name}`);
  } catch (err) {
    console.log(`   ! ${err.code}`);
  }
}
