// 把一段文本追加到文件末尾（不覆盖）。用法：node scripts/_append.mjs <file> <tailFile>
import fs from 'node:fs';

const file = (process.argv[2] || '').replace(/^["']+|["']+$/g, '');
const tailFile = (process.argv[3] || '').replace(/^["']+|["']+$/g, '');
if (!file || !tailFile) {
  console.error('用法: node scripts/_append.mjs <file> <tailFile>');
  process.exit(1);
}
const tail = fs.readFileSync(tailFile, 'utf8');
const before = fs.readFileSync(file, 'utf8');
const needsNewline = before.length > 0 && !before.endsWith('\n');
fs.writeFileSync(file, before + (needsNewline ? '\n' : '') + tail, 'utf8');
console.log(`appended ${tail.length} chars to ${file} (现在 ${fs.statSync(file).size} 字节)`);
