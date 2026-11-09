import fs from 'node:fs';
const baseDir = 'D:/nextrade/NexTrade/java-b2b2c-overseas-v5.8.61/view/pc';
const outDir = '.tmp-r27/legacy';
fs.mkdirSync(outDir, { recursive: true });
const files = [
  'components/home/CatProductNormal.vue',
  'components/home/CatProductSimple.vue',
  'components/home/CatWide.vue',
  'components/home/ImageAd.vue',
  'components/home/CategoryNew3.vue',
  'components/home/BannerSimple.vue',
  'components/home/Seckill.vue',
];
for (const rel of files) {
  const name = rel.replace(/\//g, '__');
  fs.copyFileSync(baseDir + '/' + rel, outDir + '/' + name);
  console.log('copied', name, fs.statSync(outDir + '/' + name).size);
}
