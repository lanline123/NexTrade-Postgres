// Generate docs/pc-page-parity.md from the legacy Vue tree + this app's routes.
// Read-only on sources; writes only the markdown report.
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const oldPages = path.resolve(root, '../../../java-b2b2c-overseas-v5.8.61/view/pc/pages');
const appDir = path.resolve(root, 'app');

function walk(dir, filter, out = []) {
  if (!fs.existsSync(dir)) return out;
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const full = path.join(dir, entry.name);
    if (entry.isDirectory()) walk(full, filter, out);
    else if (filter(entry.name)) out.push(full);
  }
  return out;
}

const norm = (route) =>
  route.toLowerCase().replace(/\/index$/, '').replace(/\/list$/, '').replace(/\/info$/, '').replace(/\/$/, '') || '/';

function nuxtRoute(file) {
  let r = path.relative(oldPages, file).replace(/\\/g, '/').replace(/\.vue$/, '');
  r = r.replace(/\/index$/, '').replace(/\[([^\]]+)\]/g, ':$1');
  return `/${r === 'index' ? '' : r}`;
}
function nextRoute(file) {
  let r = path.relative(appDir, file).replace(/\\/g, '/').replace(/\/?page\.tsx$/, '').replace(/\[([^\]]+)\]/g, ':$1');
  return `/${r}`.replace(/\/$/, '') || '/';
}
const kb = (n) => `${(n / 1024).toFixed(1)}KB`;

const oldFiles = walk(oldPages, (n) => n.endsWith('.vue'))
  .map((file) => ({ file, route: nuxtRoute(file) }))
  .filter((row) => !/\/src\//.test(row.route) && !/\/components\//.test(row.route) && !/^\/activity\/pc/.test(row.route))
  .sort((a, b) => a.route.localeCompare(b.route));

const newFiles = walk(appDir, (n) => n === 'page.tsx').map((file) => ({ file, route: nextRoute(file) }));
const newByNorm = new Map(newFiles.map((row) => [norm(row.route), row]));

const usedNew = new Set();
const legacyRows = oldFiles.map((row) => {
  const match = newByNorm.get(norm(row.route)) || null;
  if (match) usedNew.add(match.route);
  const oldKb = fs.statSync(row.file).size;
  const newKb = match ? fs.statSync(match.file).size : 0;
  return {
    route: row.route,
    oldFile: path.relative(oldPages, row.file).replace(/\\/g, '/'),
    oldKb,
    newRoute: match ? match.route : '—',
    newFile: match ? path.relative(root, match.file).replace(/\\/g, '/') : '—',
    newKb,
  };
});

const extras = newFiles
  .filter((row) => !usedNew.has(row.route))
  .map((row) => ({ route: row.route, file: path.relative(root, row.file).replace(/\\/g, '/'), kb: fs.statSync(row.file).size }))
  .sort((a, b) => a.route.localeCompare(b.route));

const lines = [];
lines.push('# PC 商城逐页对照表（旧版 Vue → 新版 Next.js）');
lines.push('');
lines.push('> 自动生成，别手改：`node scripts/_parity-doc.mjs`。');
lines.push('> 旧版基准：`java-b2b2c-overseas-v5.8.61/view/pc/pages`；新版：本仓库 `app/`。');
lines.push('');
lines.push(`- 旧版页面文件（去掉 src/、components/、activity/pc 内部模块后）：**${legacyRows.length}**`);
lines.push(`- 新版路由（page.tsx）：**${newFiles.length}**`);
lines.push(`- 旧版页面没有对应新版路由：**${legacyRows.filter((r) => r.newRoute === '—').length}**`);
lines.push(`- 新版独有的路由（旧版没有，含兼容别名/新增页）：**${extras.length}**`);
lines.push('');
lines.push('“新版文件”一列只是落点，不代表逐像素一致；页面主体多数在 `src/components/*Client.tsx` 里。');
lines.push('');
lines.push('## 旧版页面 → 新版路由');
lines.push('');
lines.push('| 旧版路由 | 旧版文件 | 旧版大小 | 新版路由 | 新版文件 | 新版大小 |');
lines.push('| --- | --- | --- | --- | --- | --- |');
for (const row of legacyRows) {
  lines.push(`| \`${row.route}\` | \`${row.oldFile}\` | ${kb(row.oldKb)} | \`${row.newRoute}\` | \`${row.newFile}\` | ${row.newKb ? kb(row.newKb) : '—'} |`);
}
lines.push('');
lines.push('## 新版独有路由');
lines.push('');
lines.push('| 新版路由 | 文件 | 大小 | 说明 |');
lines.push('| --- | --- | --- | --- |');
for (const row of extras) {
  let note = '新版新增';
  if (/^\/member\//.test(row.route)) note = '会员中心别名（对应旧版 /member/... 的 index/list 变体）';
  if (row.route === '/help') note = '旧版无此路由，重定向到 /article/issue';
  if (/^\/search\.v1$|index\.v1$/.test(row.route)) note = '旧路由兼容入口';
  if (/\[/.test(row.route)) note = '动态路由';
  lines.push(`| \`${row.route}\` | \`${row.file}\` | ${kb(row.kb)} | ${note} |`);
}
lines.push('');
lines.push('## 已知差异与未对齐项');
lines.push('');
lines.push('见同目录 `pc-parity-notes.md`：那份是人工维护的差异清单，本文件只保证路由级不遗漏。');
lines.push('');

fs.mkdirSync(path.resolve(root, 'docs'), { recursive: true });
fs.writeFileSync(path.resolve(root, 'docs/pc-page-parity.md'), lines.join('\n'), 'utf8');
console.log(`wrote docs/pc-page-parity.md — ${legacyRows.length} legacy pages, ${extras.length} new-only routes`);
