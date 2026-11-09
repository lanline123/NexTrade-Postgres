import fs from 'node:fs';
const file = 'src/decorate/components/modules.tsx';
let text = fs.readFileSync(file, 'utf8');

console.log('CRLF 行数:', (text.match(/\r\n/g) || []).length, ' 纯 LF 行数:', (text.match(/[^\r]\n/g) || []).length);

const START = '<span className="countdown-text">{title.countdownLabel || t(\'距结束\')}:</span>';
const END = '{countdown.split(\':\')[2]}</span>';

const NEW = [
  `<span className="countdown-text">{title.countdownLabel || t('距结束')}:</span>`,
  `{cdParts && cdParts.days > 0 ? (`,
  `<>`,
  `<span className={\`countdown-item\${String(cdParts.days).length > 2 ? ' days' : ''}\`}>{cdParts.days}</span>`,
  `<span className="countdown-separator-text">{t('天')}</span>`,
  `</>`,
  `) : null}`,
  `<span className="countdown-item">{cdParts ? pad2(cdParts.hours) : ''}</span>`,
  `<span className="countdown-separator">:</span>`,
  `<span className="countdown-item">{cdParts ? pad2(cdParts.minutes) : ''}</span>`,
  `<span className="countdown-separator">:</span>`,
  `<span className="countdown-item">{cdParts ? pad2(cdParts.seconds) : ''}</span>`,
].join('\n');

let n = 0;
for (;;) {
  const s = text.indexOf(START);
  if (s < 0) break;
  const e = text.indexOf(END, s);
  if (e < 0) break;
  console.log('段长', e + END.length - s);
  text = text.slice(0, s) + NEW + text.slice(e + END.length);
  n += 1;
}
console.log('替换', n, '处');
fs.writeFileSync(file, text, 'utf8');
