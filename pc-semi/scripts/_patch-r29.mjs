#!/usr/bin/env node
/**
 * 第 29 轮补丁：把 ContractRecommendProduct 插到 modules.tsx 的「10. 契约渲染」标记之前，
 * 并登记进 CONTRACT_COMPONENTS。
 */
import fs from 'node:fs';

const target = 'src/decorate/components/modules.tsx';
const insertFile = '.tmp-r29/insert-recommend.tsx';
const MARK = '/* ======================= 10. 契约渲染（原版 camelCase type） ======================= */';

let text = fs.readFileSync(target, 'utf8');
if (text.includes('function ContractRecommendProduct(')) {
  console.log('已存在 ContractRecommendProduct，跳过插入');
} else {
  const insert = fs.readFileSync(insertFile, 'utf8');
  const index = text.indexOf(MARK);
  if (index < 0) throw new Error('找不到「10. 契约渲染」标记，插入位置未知');
  text = `${text.slice(0, index)}${insert}${text.slice(index)}`;
  console.log('已插入 ContractRecommendProduct');
}

if (/^ {2}ContractRecommendProduct,$/m.test(text)) {
  console.log('CONTRACT_COMPONENTS 已登记，跳过');
} else {
  const anchor = '  ContractCatProductSimple,\n';
  const at = text.indexOf(anchor);
  if (at < 0) throw new Error('CONTRACT_COMPONENTS 里找不到 ContractCatProductSimple 作为锚点');
  text = `${text.slice(0, at + anchor.length)}  ContractRecommendProduct,\n${text.slice(at + anchor.length)}`;
  console.log('已登记 CONTRACT_COMPONENTS.ContractRecommendProduct');
}

fs.writeFileSync(target, text, 'utf8');
console.log(`modules.tsx -> ${Buffer.byteLength(text)} bytes`);
