import fs from 'node:fs';
import path from 'node:path';
const root = 'D:/nextrade/NexTrade/java-b2b2c-overseas-v5.8.61/view/pc';
console.log('root exists:', fs.existsSync(root));
const target = path.join(root, 'components/home/RecommendProduct.vue');
console.log('file exists:', fs.existsSync(target));
if (fs.existsSync(target)) {
  const text = fs.readFileSync(target, 'utf8');
  console.log('bytes:', text.length, 'includes cardPrice:', text.includes('cardPrice'));
}
const dirs = fs.readdirSync(root, { withFileTypes: true }).slice(0, 40);
console.log(dirs.map((d) => `${d.isDirectory() ? 'd' : 'f'}:${d.name}`).join(' '));
