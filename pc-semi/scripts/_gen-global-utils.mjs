/**
 * 从旧版 assets/css/global.less 逐字搬「通用工具类」区块到 app/pc-global-utils.scss。
 * 起因：新版从未搬过 global.less 的 .container / .flex / .align-* / .justify-* / .red /
 * .hand-pointer / .ellipsis / .sticky-element，而这些类被旧版每个页面当基础设施用，
 * 换成旧版类名后没有对应规则的页面就会塌成 block 排列。
 * 只做一件事：按行区间原样复制，不改一个字符（`//` 注释在 scss 里同样合法）。
 *
 * 区间：
 *   246–364  .container / 颜色 / 按钮 / .flex / .justify-* / .align-* / .ellipsis / .sticky-element
 *   366–419  .icon-star 家族（评分星星雪碧图；商品详情评分、评价卡片都在用）
 *            雪碧图由 scripts/copy-icon-star.mjs 从旧版 assets/images/common/icon-star.png 拷进 public
 */
import fs from 'node:fs';

const SRC = 'D:/nextrade/NexTrade/java-b2b2c-overseas-v5.8.61/view/pc/assets/css/global.less';
const OUT = 'app/pc-global-utils.scss';
const RANGES = [
  { from: 246, to: 364 }, // .container … .sticky-element
  { from: 366, to: 419 }, // .icon-star … .icon-star-0
];

const lines = fs.readFileSync(SRC, 'utf8').split('\n');
const body = RANGES.map(({ from, to }) => lines.slice(from - 1, to).join('\n').replace(/\s+$/, '')).join('\n\n');

const header = `/**
 * 自动生成，请勿手改。生成器：scripts/_gen-global-utils.mjs
 * 来源：旧版 ${SRC}
 *   第 246–364 行（原样复制）
 *   第 366–419 行（原样复制）
 * 内容：.container / .red / .flex / .flex-column / .justify-* / .align-* / .flex-wrap /
 *       .hand-pointer / .ellipsis / .ellipsis2 / .sticky-element / .icon-star 家族
 */
`;

fs.writeFileSync(OUT, header + body + '\n', 'utf8');
console.log(`${OUT}: ${(header + body).length} bytes, ${body.split('\n').length} 行`);
