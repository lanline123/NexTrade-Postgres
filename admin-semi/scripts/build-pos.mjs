#!/usr/bin/env node
/**
 * 生成可部署到站点根 /pos 的收银台静态目录。
 *
 * 输入：o2o_admin 5.8.60 打包产物（无源码），默认
 *   D:\tigshop\前端页面\5.8.60\o2o_admin\admin
 * 可用 POS_ASSETS_DIR 指向同结构目录。
 *
 * 输出：<admin-semi>/pos-dist/（index.html + assets/），
 * 上传到站点根目录下的 pos/ 即可通过 https://<host>/pos 访问。
 */
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const POS_BASE_LITERAL = 'VITE_BASE_DIR:`admin`';
const POS_BASE_PATCHED = 'VITE_BASE_DIR:``';
const PUBLIC_PATH = '/pos';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const srcDir = process.env.POS_ASSETS_DIR || 'D:/nextrade/前端页面/5.8.60/o2o_admin/admin';
const srcAssets = path.join(srcDir, 'assets');
const outDir = path.join(root, 'pos-dist');
const outAssets = path.join(outDir, 'assets');

if (!fs.existsSync(srcAssets)) {
  console.error(`[build-pos] 找不到 POS 产物目录: ${srcAssets}`);
  console.error('[build-pos] 请用 POS_ASSETS_DIR 指定 o2o_admin/admin 目录');
  process.exit(1);
}

fs.rmSync(outDir, { recursive: true, force: true });
fs.mkdirSync(outAssets, { recursive: true });

// 逐文件复制：Windows 上 fs.cpSync 复制该产物会静默退出（exit 127），不要改回 cpSync。
function copyTree(from, to) {
  for (const entry of fs.readdirSync(from, { withFileTypes: true })) {
    const source = path.join(from, entry.name);
    const target = path.join(to, entry.name);
    if (entry.isDirectory()) {
      fs.mkdirSync(target, { recursive: true });
      copyTree(source, target);
    } else {
      fs.copyFileSync(source, target);
    }
  }
}
copyTree(srcAssets, outAssets);

const favicon = path.join(srcDir, 'favicon.ico');
if (fs.existsSync(favicon)) fs.copyFileSync(favicon, path.join(outDir, 'favicon.ico'));

const names = fs.readdirSync(outAssets);
const bySizeDesc = (a, b) => fs.statSync(path.join(outAssets, b)).size - fs.statSync(path.join(outAssets, a)).size;

const entry = names.filter((file) => /^index-.*\.js$/.test(file)).sort(bySizeDesc)[0];
const cssFiles = names.filter((file) => /^index-.*\.css$/.test(file)).sort(bySizeDesc).slice(0, 2);

if (!entry) {
  console.error('[build-pos] 产物中未找到 index-*.js 入口');
  process.exit(1);
}

let patched = 0;
for (const file of names.filter((name) => name.endsWith('.js'))) {
  const filePath = path.join(outAssets, file);
  const code = fs.readFileSync(filePath, 'utf8');
  if (!code.includes(POS_BASE_LITERAL)) continue;
  fs.writeFileSync(filePath, code.split(POS_BASE_LITERAL).join(POS_BASE_PATCHED));
  patched++;
}

const html = `<!doctype html>
<html lang="zh-CN">
<head>
  <meta charset="UTF-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1, maximum-scale=1, user-scalable=no" />
  <title>收银台</title>
  ${cssFiles.map((file) => `<link rel="stylesheet" href="${PUBLIC_PATH}/assets/${file}" />`).join('\n  ')}
</head>
<body>
  <div id="app"></div>
  <script type="module" src="${PUBLIC_PATH}/assets/${entry}"></script>
</body>
</html>
`;
fs.writeFileSync(path.join(outDir, 'index.html'), html);

const totalFiles = fs.readdirSync(outAssets).length;
console.log(`[build-pos] 输出: ${outDir}`);
console.log(`[build-pos] 入口: ${entry}`);
console.log(`[build-pos] CSS: ${cssFiles.join(', ')}`);
console.log(`[build-pos] 资源: ${totalFiles} 个，base 补丁: ${patched} 个 chunk`);
console.log('[build-pos] 部署: 将 pos-dist 内文件上传到站点根目录的 pos/ 下');
console.error('[build-pos] 提示: nginx 需为 /pos 兜底 —— location ^~ /pos { try_files $uri $uri/ /pos/index.html; }');