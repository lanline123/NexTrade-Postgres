// 把 .tmp-r28/insert-simple.tsx 插进 modules.tsx 的「10. 契约渲染」段之前，并登记进 CONTRACT_COMPONENTS。
import fs from 'node:fs';

const target = 'src/decorate/components/modules.tsx';
const insert = '.tmp-r28/insert-simple.tsx';

const src = fs.readFileSync(target, 'utf8');
const add = fs.readFileSync(insert, 'utf8');

if (src.includes('\r')) throw new Error('modules.tsx 不是纯 LF，先确认再改');
if (src.includes('function ContractCatProductSimple')) throw new Error('已插入过，勿重复');

const marker = '/* ======================= 10. 契约渲染（原版 camelCase type） ======================= */';
const hits = src.split(marker).length - 1;
if (hits !== 1) throw new Error(`marker 命中 ${hits} 次，应恰好 1 次`);

const before = src.split('\n').length;
let out = src.replace(marker, add + marker);

const regAnchor = '  ContractImageAd,\n';
if (out.split(regAnchor).length - 1 !== 1) throw new Error('注册表锚点不唯一');
out = out.replace(regAnchor, regAnchor + '  ContractCatProductSimple,\n');

fs.writeFileSync(target, out, 'utf8');
const after = out.split('\n').length;
console.log(`modules.tsx: ${before} -> ${after} 行 (+${after - before})`);
