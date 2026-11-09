import fs from 'node:fs';
const file = (process.argv[2] || 'app/semi.css').replace(/^["']+|["']+$/g, '');
const src = fs.readFileSync(file, 'utf8');
const needle = 'font-family';
let i = 0;
let n = 0;
while ((i = src.indexOf(needle, i)) !== -1 && n < 20) {
  n += 1;
  // 找这条声明所属的选择器：往前找最近一个 } 或 ; 之后的 { 
  const prevOpen = src.lastIndexOf('{', i);
  const prevClose = src.lastIndexOf('}', i);
  const selStart = Math.max(src.lastIndexOf('}', prevOpen), src.lastIndexOf(';', prevOpen), 0);
  const sel = src.slice(selStart + 1, prevOpen).trim();
  const decl = src.slice(i, src.indexOf(';', i) + 1);
  console.log(`#${n} 选择器: ${sel.slice(0, 120)}`);
  console.log(`    声明: ${decl.slice(0, 200)}\n`);
  i += needle.length;
}
console.log(`共 ${n} 处 font-family；文件 ${src.length} 字节`);
