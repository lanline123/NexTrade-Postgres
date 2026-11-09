// 找 icon-star 区块的行号：node scripts/_ls-lines.mjs
import fs from 'node:fs';

const SRC = 'D:/nextrade/NexTrade/java-b2b2c-overseas-v5.8.61/view/pc/assets/css/global.less';
const lines = fs.readFileSync(SRC, 'utf8').split('\n');
lines.forEach((line, index) => {
  if (/icon-star|商品详情页评分星星|^\/\/勾选|\.icon-gou/.test(line)) {
    console.log(`${index + 1}| ${line}`);
  }
});
console.log('total lines:', lines.length);
