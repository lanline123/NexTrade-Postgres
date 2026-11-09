import fs from 'node:fs';
const dir = process.argv[2] || '.tmp-r23';
fs.mkdirSync(dir, { recursive: true });
console.log(`已创建 ${dir}`);
