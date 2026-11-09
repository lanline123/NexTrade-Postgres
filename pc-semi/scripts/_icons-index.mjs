import fs from 'node:fs';
const dir = 'node_modules/@icon-park/react';
const t = fs.readFileSync(`${dir}/es/index.js`, 'utf8');
console.log('index.js bytes', t.length);
console.log(t.slice(0, 600));
// 导出名：形如 var Xxx = ... 或 export { A as B }
const exportsBlock = t.match(/export\s*\{([\s\S]{0,4000})\}/) || t.match(/export\s*\*\s*from[\s\S]{0,400}/);
console.log('export block found:', Boolean(exportsBlock));
if (exportsBlock) console.log(exportsBlock[1].slice(0, 1200));
