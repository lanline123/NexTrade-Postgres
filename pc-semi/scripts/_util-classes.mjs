// 旧版 global.less 里有哪些 mt/mb 工具类：node scripts/_util-classes.mjs
import fs from 'node:fs';

const file = 'D:/nextrade/NexTrade/java-b2b2c-overseas-v5.8.61/view/pc/assets/css/global.less';
const text = fs.readFileSync(file, 'utf8');
const names = [...new Set([...text.matchAll(/^\s*\.((?:m[tblr]|p[tblr])\d{1,2})\s*(?:,|\{)/gm)].map((m) => m[1]))].sort();
console.log('utility classes in global.less:', names.join(', ') || '(none)');
const iMt = text.split(/\r?\n/).filter((line) => /\.i-mt\b/.test(line)).slice(0, 4);
console.log('.i-mt lines:', iMt.join(' || ') || '(none)');
