import fs from 'node:fs';
const file = 'src/decorate/components/modules.tsx';
let text = fs.readFileSync(file, 'utf8');
const log = [];

// ── 1. 旧版 CountDown 默认插槽的 days/hours/minutes/seconds 拆分 ──
const marker = '/* ======================= 3. pc-home-quick-nav ======================= */';
const helper = `/** 旧版 CountDown 默认插槽的 days/hours/minutes/seconds：把剩余毫秒拆成四段。 */
export function countdownParts(remainingMs: number) {
  const total = Math.max(0, Math.floor(remainingMs / 1000));
  return {
    days: Math.floor(total / 86400),
    hours: Math.floor((total % 86400) / 3600),
    minutes: Math.floor((total % 3600) / 60),
    seconds: total % 60,
  };
}

/** 两位补零（旧版 CountDown 内部就是 padStart(2, "0")）。 */
function pad2(n: number) {
  return String(n).padStart(2, '0');
}

`;
if (text.includes('export function countdownParts')) {
  log.push('跳过：countdownParts 已存在');
} else if (!text.includes(marker)) {
  console.error('没找到第 3 节注释');
  process.exit(1);
} else {
  text = text.replace(marker, helper + marker);
  log.push('已加 countdownParts / pad2');
}

// ── 2. 倒计时结构：旧版 PcSeckillLegacy.vue:85-91 / PcGrouponLegacy.vue:81-89 ──
// 逐行拼出旧串（含缩进），不做模糊匹配，避免上次把中间内容吃掉。
const OLD = (ind) =>
  [
    `<span className="countdown-text">{title.countdownLabel || t('距结束')}:</span>`,
    `${ind}<span className="countdown-item">{countdown.split(':')[0]}</span>`,
    `${ind}<span className="countdown-separator">:</span>`,
    `${ind}<span className="countdown-item">{countdown.split(':')[1]}</span>`,
    `${ind}<span className="countdown-separator">:</span>`,
    `${ind}<span className="countdown-item">{countdown.split(':')[2]}</span>`,
  ].join('\n');

const NEW = (ind) =>
  [
    `<span className="countdown-text">{title.countdownLabel || t('距结束')}:</span>`,
    `{cdParts && cdParts.days > 0 ? (`,
    `${ind}  <>`,
    `${ind}    <span className={\`countdown-item\${String(cdParts.days).length > 2 ? ' days' : ''}\`}>{cdParts.days}</span>`,
    `${ind}    <span className="countdown-separator-text">{t('天')}</span>`,
    `${ind}  </>`,
    `${ind}) : null}`,
    `${ind}<span className="countdown-item">{cdParts ? pad2(cdParts.hours) : ''}</span>`,
    `${ind}<span className="countdown-separator">:</span>`,
    `${ind}<span className="countdown-item">{cdParts ? pad2(cdParts.minutes) : ''}</span>`,
    `${ind}<span className="countdown-separator">:</span>`,
    `${ind}<span className="countdown-item">{cdParts ? pad2(cdParts.seconds) : ''}</span>`,
  ].join('\n');

let hits = 0;
for (let n = 28; n >= 4; n -= 1) {
  const ind = ' '.repeat(n);
  const from = OLD(ind);
  while (text.includes(from)) {
    text = text.replace(from, NEW(ind));
    hits += 1;
  }
}
log.push(`倒计时结构替换：${hits} 处`);

// ── 3. 两个块里各加一行 cdParts ──
let declHits = 0;
text = text.replace(/( *const countdown = now && remaining > 0\n[^\n]*\n *: '';\n)/g, (m) => {
  declHits += 1;
  const ind = m.match(/^( *)/)[1];
  return m + `${ind}const cdParts = remaining > 0 ? countdownParts(remaining) : null;\n`;
});
log.push(`cdParts 声明：${declHits} 处`);

fs.writeFileSync(file, text, 'utf8');
console.log(log.join('\n'));
