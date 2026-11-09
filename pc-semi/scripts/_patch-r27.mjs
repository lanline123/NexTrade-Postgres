// 把 .tmp-r27/insert-components.tsx 插进 modules.tsx 的「10. 契约渲染」段之前，
// 并把两个新组件登记进 CONTRACT_COMPONENTS。
import fs from 'node:fs';

const target = 'src/decorate/components/modules.tsx';
const insert = '.tmp-r27/insert-components.tsx';

const src = fs.readFileSync(target, 'utf8');
const add = fs.readFileSync(insert, 'utf8');
if (!src.includes('\r')) {
  // expect pure LF
} else {
  throw new Error('modules.tsx 不是纯 LF，先确认再改');
}

const marker = '/* ======================= 10. 契约渲染（原版 camelCase type） ======================= */';
const hits = src.split(marker).length - 1;
if (hits !== 1) throw new Error(`marker 命中 ${hits} 次，应恰好 1 次`);
if (src.includes('function ContractCatProductNormal')) throw new Error('已插入过，勿重复');

const before = src.split('\n').length;
let out = src.replace(marker, add + marker);

const regAnchor = '  ContractCatProductWide,\n';
if (out.split(regAnchor).length - 1 !== 1) throw new Error('注册表锚点不唯一');
out = out.replace(regAnchor, regAnchor + '  ContractCatProductNormal,\n  ContractImageAd,\n');

fs.writeFileSync(target, out, 'utf8');
const after = out.split('\n').length;
console.log(`modules.tsx: ${before} -> ${after} 行 (+${after - before})`);
