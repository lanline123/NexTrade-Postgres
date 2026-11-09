// scripts/_tail-replace.mjs <spec.json>
// 把某个文件从第一个 marker 起的内容整段换成 tail 文件的内容（保留 marker 之前的部分）。
// spec.json = { "file": "...", "marker": "...", "tail": "..." }
// 用 JSON 传参是为了绕开命令执行器不剥引号、且会吃掉 > 和 & 的问题。
import fs from 'node:fs';

const specPath = process.argv[2];
if (!specPath) {
  console.error('usage: _tail-replace.mjs <spec.json>');
  process.exit(2);
}
const spec = JSON.parse(fs.readFileSync(specPath, 'utf8'));
const { file, marker, tail } = spec;
if (!file || !marker || !tail) {
  console.error('spec needs file / marker / tail');
  process.exit(2);
}
const source = fs.readFileSync(file, 'utf8');
const index = source.indexOf(marker);
if (index < 0) {
  console.error(`marker not found in ${file}: ${JSON.stringify(marker)}`);
  process.exit(1);
}
if (source.indexOf(marker, index + marker.length) >= 0) {
  console.error(`marker appears more than once in ${file}`);
  process.exit(1);
}
const next = source.slice(0, index) + fs.readFileSync(tail, 'utf8');
fs.writeFileSync(file, next);
console.log(`${file}: cut at ${index}, now ${Buffer.byteLength(next)} bytes`);
