/**
 * 把 pc-semi 的优惠券单一数据源 SCSS 生成为 admin-semi 可直接 import 的 CSS。
 *
 * 背景：
 *   admin-semi 只装了 less、没有 sass（离线环境也不能新增依赖），无法直接 import .scss。
 *   而优惠券样式必须两端共用同一份源码，所以在构建期做一次 SCSS → CSS 转译。
 *   产物 admin-semi/src/pages/decoratePc/previews/pcCoupon.generated.css 只读不手改。
 *
 * sass 的来源（按顺序探测，任一可用即可）：
 *   1. 环境变量 SASS_BIN 指向 sass.js 或 sass 可执行文件
 *   2. 本仓库 pc-semi/node_modules/sass
 *   3. 上游构建树 D:\\tigshop\\NexTrade\\rust\\web\\pc-semi/node_modules/sass
 *   4. npx sass（可能触发联网，放最后）
 *
 * 用法：
 *   node scripts/gen-coupon-css.mjs          # 生成
 *   node scripts/gen-coupon-css.mjs --check  # 只校验产物是否与源码同步
 */
import fs from 'node:fs';
import path from 'node:path';
import { execFileSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const here = path.dirname(fileURLToPath(import.meta.url));
const adminRoot = path.resolve(here, '..');
const repoRoot = path.resolve(adminRoot, '..');

const SRC = path.join(repoRoot, 'pc-semi/src/decorate/coupon/coupon.scss');
const OUT = path.join(adminRoot, 'src/pages/decoratePc/previews/pcCoupon.generated.css');

const BANNER = `/* ------------------------------------------------------------------
 * 自动生成，请勿手改。
 * 源码：pc-semi/src/decorate/coupon/coupon.scss（PC 与后台共用的单一数据源）
 * 生成：cd admin-semi && node scripts/gen-coupon-css.mjs
 *
 * 为什么要有这份副本：admin-semi 只装了 less、没有 sass，无法直接 import .scss。
 * 改样式请改上面的源码后重新生成，不要在这里改。
 * ---------------------------------------------------------------- */
`;

function resolveSass() {
  const candidates = [
    process.env.SASS_BIN,
    path.join(repoRoot, 'pc-semi/node_modules/sass/sass.js'),
    path.join(repoRoot, 'admin-semi/node_modules/sass/sass.js'),
    'D:/tigshop/NexTrade/rust/web/pc-semi/node_modules/sass/sass.js',
  ].filter(Boolean);
  for (const c of candidates) {
    if (fs.existsSync(c)) return { kind: 'js', file: c };
  }
  return null;
}

function compile() {
  const sass = resolveSass();
  if (!sass) {
    throw new Error(
      '找不到 sass。请设置环境变量 SASS_BIN 指向 sass.js，或确认 pc-semi/node_modules/sass 存在。',
    );
  }
  const raw = execFileSync(process.execPath, [sass.file, '--no-source-map', '--style=expanded', SRC], {
    encoding: 'utf-8',
    maxBuffer: 8 * 1024 * 1024,
  });
  // sass 会加 @charset 与多行注释；去掉 charset，把顶部注释换成说明性 BANNER
  let css = raw.replace(/^@charset\s+"[^"]*";\s*/, '');
  css = css.replace(/^\/\*\*[\s\S]*?\*\/\s*/, '');
  return `${BANNER}\n${css.trim()}\n`;
}

const generated = compile();

if (process.argv.includes('--check')) {
  const existing = fs.existsSync(OUT) ? fs.readFileSync(OUT, 'utf-8') : '';
  const norm = (s) => s.replace(/\r\n/g, '\n').trim();
  if (norm(existing) !== norm(generated)) {
    console.error('[gen-coupon-css] 产物已过期，请运行：cd admin-semi && node scripts/gen-coupon-css.mjs');
    process.exit(1);
  }
  console.log('[gen-coupon-css] 产物与源码一致');
  process.exit(0);
}

fs.writeFileSync(OUT, generated, 'utf-8');
const rel = (p) => path.relative(repoRoot, p).replace(/\\/g, '/');
console.log(`[gen-coupon-css] ${rel(SRC)}`);
console.log(`[gen-coupon-css]   -> ${rel(OUT)} (${Buffer.byteLength(generated, 'utf-8')} bytes)`);