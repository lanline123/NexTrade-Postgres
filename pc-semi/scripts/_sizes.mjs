// 打印若干文件的行数与字节数。用法：node scripts/_sizes.mjs f1 f2 ...
import fs from 'node:fs';

for (const f of process.argv.slice(2)) {
  try {
    const s = fs.readFileSync(f, 'utf8');
    console.log(`${String(s.length).padStart(8)} B  ${String(s.split('\n').length).padStart(6)} L  ${f}`);
  } catch (err) {
    console.log(`       !  ${err.code}  ${f}`);
  }
}
