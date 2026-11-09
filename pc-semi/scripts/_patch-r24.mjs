#!/usr/bin/env node
/**
 * 第 24 轮：把 src/decorate/components/modules.tsx 里三个「旧版复刻」组件的函数体
 * 整段换成逐字照抄 components/home/*.vue 的实现。
 *
 * 为什么用行号而不是字符串替换：这个文件 3900+ 行，上一轮一次字符串拼接删掉过 234 行。
 * 这里每个边界行都先断言，断言不过就直接退出、不写文件。
 *
 * 顺手把 contract.ts 的 adaptPcCustomAd2 补上 picList/picList2/picList3 原始数组
 * （CustomAd2.vue 是按 picList2[0..4] / picList3 下标取的，聚合后的 groups 不够用）。
 */
import fs from 'node:fs';

const MODULES = 'src/decorate/components/modules.tsx';
const CONTRACT = 'src/decorate/components/contract.ts';

const read = (p) => fs.readFileSync(p, 'utf8').split('\n');

// ---------- modules.tsx ----------
const lines = read(MODULES);

/** 1-based 行号 → 必须满足的断言 */
const expect = [
  [3547, '/** pcBanner bannerStyle=1'],
  [3548, 'function ContractBanner({ module }: ModuleProps) {'],
  [3585, '}'],
  [3748, '/** pcCustomAd1 / pcCustomAd2 共用'],
  [3749, 'function ContractCustomAd({ module }: ModuleProps) {'],
  [3766, '}'],
  [3768, '/** pcCustomAd1 专用'],
  [3771, 'function ContractCustomAd1({ module }: ModuleProps) {'],
  [3866, '}'],
];

for (const [lineNo, prefix] of expect) {
  const actual = lines[lineNo - 1] ?? '';
  if (!actual.startsWith(prefix)) {
    console.error(`断言失败：第 ${lineNo} 行应以「${prefix}」开头，实际是「${actual}」`);
    process.exit(1);
  }
}

const body = (file) => fs.readFileSync(file, 'utf8').replace(/\n$/, '').split('\n');

const replacements = [
  { from: 3768, to: 3866, lines: body('.tmp-r24/customad1.tsx.txt') },
  { from: 3748, to: 3766, lines: body('.tmp-r24/customad2.tsx.txt') },
  { from: 3547, to: 3585, lines: body('.tmp-r24/banner.tsx.txt') },
];

let out = lines.slice();
for (const { from, to, lines: next } of replacements) {
  out = [...out.slice(0, from - 1), ...next, ...out.slice(to)];
}

fs.writeFileSync(MODULES, out.join('\n'), 'utf8');
console.log(`modules.tsx: ${lines.length} 行 → ${out.length} 行`);

// ---------- contract.ts ----------
let contract = fs.readFileSync(CONTRACT, 'utf8');
const before = `      groups: [picList(m, 'picList'), picList(m, 'picList2'), picList(m, 'picList3')],
      links: toLinks(m.linkList),`;
const after = `      // CustomAd2.vue 按 picList2[0..4] / picList3 下标逐格取图，必须给原始数组
      picList: picList(m, 'picList'),
      picList2: picList(m, 'picList2'),
      picList3: picList(m, 'picList3'),
      groups: [picList(m, 'picList'), picList(m, 'picList2'), picList(m, 'picList3')],
      links: toLinks(m.linkList),`;
if (!contract.includes(before)) {
  console.error('断言失败：contract.ts 里没找到 adaptPcCustomAd2 的 groups/links 两行');
  process.exit(1);
}
contract = contract.replace(before, after);
fs.writeFileSync(CONTRACT, contract, 'utf8');
console.log('contract.ts: adaptPcCustomAd2 已补 picList/picList2/picList3');
