import fs from 'node:fs';
const file = 'src/decorate/components/modules.tsx';
let text = fs.readFileSync(file, 'utf8');
const before = text;

// 1) 加 splitWanSuffix（照抄 PcHomeHero.vue:1020-1023）
const helperAnchor = `  const needWideAssets = assetMoney(profile?.balance).length >= 7 || assetNumber(profile?.points).length >= 7;`;
if (!text.includes(helperAnchor)) {
  console.error('needWideAssets 锚点没找到');
  process.exit(1);
}
text = text.replace(
  helperAnchor,
  `  /** 旧版 PcHomeHero.vue:1020 splitWanSuffix：万/亿/K/M/B 后缀单独一个 span */
  const splitWanSuffix = (value: string): { value: string; suffix: string } => {
    const match = value.match(/^(.*?)([万亿KMB])$/);
    return match ? { value: match[1], suffix: match[2] } : { value, suffix: '' };
  };
  const balanceDisplay = splitWanSuffix(assetMoney(profile?.balance));
  const pointsDisplay = splitWanSuffix(assetNumber(profile?.points));
${helperAnchor}`,
);

// 2) 余额 / 积分拆成 value + suffix 两段（旧版 271/272、278/279 行）
const moneyLine = `                  <div className="dec-pc-home-hero__asset-num">{assetMoney(profile?.balance)}</div>`;
if (!text.includes(moneyLine)) {
  console.error('余额行没找到');
  process.exit(1);
}
text = text.replace(
  moneyLine,
  `                  <div className="dec-pc-home-hero__asset-num">
                    <span className="dec-pc-home-hero__asset-num-value">{balanceDisplay.value}</span>
                    {balanceDisplay.suffix ? (
                      <span className="dec-pc-home-hero__asset-num-suffix">{balanceDisplay.suffix}</span>
                    ) : null}
                  </div>`,
);

const pointsLine = `                  <div className="dec-pc-home-hero__asset-num">{assetNumber(profile?.points)}</div>`;
if (!text.includes(pointsLine)) {
  console.error('积分行没找到');
  process.exit(1);
}
text = text.replace(
  pointsLine,
  `                  <div className="dec-pc-home-hero__asset-num">
                    <span className="dec-pc-home-hero__asset-num-value">{pointsDisplay.value}</span>
                    {pointsDisplay.suffix ? (
                      <span className="dec-pc-home-hero__asset-num-suffix">{pointsDisplay.suffix}</span>
                    ) : null}
                  </div>`,
);

// 3) seckill-more → seckill-more-link（旧版 PcSeckillLegacy.vue 第 101 行）
const beforeCount = (text.match(/className="seckill-more"/g) || []).length;
text = text.replace(/className="seckill-more"/g, 'className="seckill-more-link"');

if (text === before) {
  console.error('没有改动');
  process.exit(1);
}
fs.writeFileSync(file, text, 'utf8');
console.log(`已更新 modules.tsx（seckill-more 改名 ${beforeCount} 处）`);
