#!/usr/bin/env node
/**
 * 会员中心公共样式（旧版 assets/css/member/member.css）逐条对账。
 *
 * 旧版每个会员页面 <style scoped> 里都写了 `@import "/assets/css/member/member"`，
 * 所以这块公共样式（.info-row / .title-or-tabs / .tig-tabs / .tig-input / .tig-button /
 * .no-result / .el-page …）在旧站是每页都有的。新版是手写搬运，容易漏。
 *
 * 本脚本把 member.css 里每条规则的选择器抽出来，逐条到 out/ 的产物 CSS 里找：
 *   HIT   产物里有
 *   MISS  产物里没有（= 页面用了这个 class 却没样式，属于真实欠账）
 * 只读，不改任何文件。
 */
import fs from 'node:fs';
import path from 'node:path';

const LEGACY = 'D:/nextrade/NexTrade/java-b2b2c-overseas-v5.8.61/view/pc/assets/css/member/member.css';
const OUT = path.join(process.cwd(), 'out');

function productCss() {
  const files = [];
  const walk = (dir) => {
    for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
      const full = path.join(dir, entry.name);
      if (entry.isDirectory()) walk(full);
      else if (entry.name.endsWith('.css')) files.push(fs.readFileSync(full, 'utf8'));
    }
  };
  if (!fs.existsSync(OUT)) return { text: '', count: 0 };
  walk(OUT);
  return { text: files.join('\n'), count: files.length };
}

const source = fs.readFileSync(LEGACY, 'utf8').replace(/\/\*[\s\S]*?\*\//g, '');
const { text: bundle, count } = productCss();

// 逐条抽选择器：member.css 是已编译的扁平 CSS，没有 @media
const selectors = [];
let buffer = '';
for (const ch of source) {
  if (ch === '{') {
    const head = buffer.trim();
    if (head) selectors.push(head);
    buffer = '';
  } else if (ch === '}') {
    buffer = '';
  } else {
    buffer += ch;
  }
}

const seen = new Set();
const rows = [];
for (const selector of selectors) {
  if (selector.startsWith('@')) continue;
  for (const part of selector.split(',')) {
    // 去掉伪类 / :deep 包裹后取"能被搜到"的最长一段
    const clean = part
      .replace(/:deep\(([^()]*)\)/g, '$1')
      .replace(/\s+/g, ' ')
      .trim();
    if (!clean || seen.has(clean)) continue;
    seen.add(clean);
    // 产物 CSS 会被压成没有空格的形态，所以把选择器里的空格也去掉再比
    const probe = clean.replace(/\s+/g, '').replace(/::?(hover|focus|active|last-child|first-child|before|after)/g, '');
    const hit = probe.length > 3 && bundle.replace(/\s+/g, '').includes(probe);
    rows.push({ selector: clean, hit });
  }
}

const missing = rows.filter((row) => !row.hit);
console.log(`扫描 ${count} 个产物 CSS，master.less/member.css 共 ${rows.length} 条选择器`);
console.log(`HIT ${rows.length - missing.length} / MISS ${missing.length}\n`);
for (const row of missing) console.log(`  MISS  ${row.selector}`);
