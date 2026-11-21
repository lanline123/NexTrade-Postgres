#!/usr/bin/env node
/**
 * 给静态面包写入 version.json 版本戳（admin 自助热更新 P1，我方新增）。
 *
 * 在各面 CI 的「打包」步骤之前运行（workflow 增补片段见
 * hot-update-design/IMPL-REPORT.md），把 {surface, productVersion, gitSha,
 * build, buildTime, stamp} 写进包根。后端状态接口优先读它判版本，
 * 缺失时才降级用 index.html 入口指纹 / DEPLOY_INFO（口径如实标注）。
 *
 * stamp 定义（与后端 read_surface_stamp / make_manifest 口径一致）：
 * - admin → index.html 引用的 index-<hash>.js 文件名
 * - h5    → index.html 引用的 app.<hash>.js 文件名
 * - pc    → .next/BUILD_ID 内容
 *
 * 用法：
 *   node write_version_json.mjs --dir admin-semi/dist --surface admin
 *   node write_version_json.mjs --dir /tmp/pc-out --surface pc
 * 环境：GITHUB_SHA / GITHUB_RUN_NUMBER（CI 自动有）；产品版本默认读
 * rust/crates/common/src/version.rs 的 PRODUCT_VERSION，可用 --version 覆盖。
 */
import { readFileSync, writeFileSync, existsSync } from 'node:fs';
import { join, resolve } from 'node:path';

function arg(name) {
  const i = process.argv.indexOf(`--${name}`);
  return i >= 0 ? process.argv[i + 1] : undefined;
}

const dir = arg('dir');
const surface = arg('surface');
if (!dir || !['admin', 'pc', 'h5'].includes(surface ?? '')) {
  console.error('用法: write_version_json.mjs --dir <包目录> --surface admin|pc|h5 [--version x.y.z]');
  process.exit(2);
}

const repoRoot = resolve(new URL('.', import.meta.url).pathname, '..', '..');
let version = arg('version');
if (!version) {
  const vf = join(repoRoot, 'rust/crates/common/src/version.rs');
  const text = existsSync(vf) ? readFileSync(vf, 'utf8') : '';
  const m = text.match(/PRODUCT_VERSION:\s*&str\s*=\s*"([^"]+)"/);
  version = m ? m[1] : 'unknown';
}

function extractEntry(html, prefix, suffix) {
  const i = html.indexOf(prefix);
  if (i < 0) return '';
  const tail = html.slice(i);
  const j = tail.indexOf(suffix);
  if (j < 0) return '';
  const cand = tail.slice(0, j + suffix.length);
  return /^[A-Za-z0-9\-_.]+$/.test(cand) ? cand : '';
}

let stamp = '';
if (surface === 'pc') {
  const bid = join(dir, '.next', 'BUILD_ID');
  if (existsSync(bid)) stamp = readFileSync(bid, 'utf8').trim();
} else {
  const htmlPath = join(dir, 'index.html');
  if (existsSync(htmlPath)) {
    const html = readFileSync(htmlPath, 'utf8');
    stamp = extractEntry(html, surface === 'admin' ? 'index-' : 'app.', '.js');
  }
}
if (!stamp) {
  console.error(`警告：${surface} 包内读不到版本戳（stamp 为空），仍写入 version.json`);
}

const payload = {
  surface,
  productVersion: version,
  gitSha: process.env.GITHUB_SHA || 'unknown',
  build: process.env.GITHUB_RUN_NUMBER || 'unknown',
  buildTime: Math.floor(Date.now() / 1000),
  stamp,
};
writeFileSync(join(dir, 'version.json'), JSON.stringify(payload, null, 1) + '\n');
console.log(`version.json 已写入 ${dir}：${JSON.stringify(payload)}`);
