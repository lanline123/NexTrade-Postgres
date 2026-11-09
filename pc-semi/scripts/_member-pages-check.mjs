#!/usr/bin/env node
/**
 * 会员中心剩余子页：检查旧版选择器是否真的进了编译产物 CSS。
 *
 * 用法：node scripts/_member-pages-check.mjs
 * 说明：会员页未登录会被 middleware 重定向到登录页，导出的 HTML 是登录页（见 findings fnd_5），
 *      所以这里只验证「源码照抄的旧版样式」有没有被编译进 out/_next/static/css/*.css。
 */
import fs from 'node:fs';
import path from 'node:path';

const ROOT = process.cwd();
const CSS_DIR = path.join(ROOT, 'out', '_next', 'static', 'css');

if (!fs.existsSync(CSS_DIR)) {
  console.error('[fail] 找不到 out/_next/static/css —— 先跑 npm run build');
  process.exit(1);
}

const cssFiles = fs.readdirSync(CSS_DIR).filter((f) => f.endsWith('.css'));
const bundle = cssFiles.map((f) => fs.readFileSync(path.join(CSS_DIR, f), 'utf8'));
const all = bundle.join('\n');

// [选择器, 来自哪个旧版文件]
const EXPECT = [
  // 地址簿
  ['.mp-address-list', 'member/address/List.vue'],
  ['.card-list .card', 'member/address/List.vue'],
  ['.mp-address-edit', 'member/address/EditRegion.vue'],
  // 优惠券
  ['.mp-coupon-list', 'member/coupon/List.vue'],
  ['.mp-coupon-card', 'components/member/coupon/CouponCard.vue'],
  ['.coupon-container', 'components/member/coupon/CouponCard.vue'],
  ['.coupon-off-text', 'components/member/coupon/CouponCard.vue'],
  ['.use-btn', 'components/member/coupon/CouponCard.vue'],
  // 收藏
  ['.mp-collect-product', 'member/collectProduct/List.vue'],
  ['.mp-collect-shop', 'member/collectShop/List.vue'],
  ['.collect-list-content', 'member/collectShop/List.vue'],
  // 注意：member/group/List.vue 的 <style> 只有一行 `@import member`，没有自己的规则，
  // 因此生成器不会产出 .mp-group-list 包裹层（页面只用共享的 .info-row/.title-or-tabs）。
  // 积分 / 余额
  ['.mp-point-list', 'member/point/List.vue'],
  ['.mp-account-detail', 'member/account/Detail.vue'],
  ['.account-detail-content', 'member/account/Detail.vue'],
  ['.mp-account-log', 'member/account/Log.vue'],
  ['.mp-account-deposit', 'member/account/Deposit.vue'],
  ['.mp-account-card', 'member/account/CardManagement.vue'],
  ['.mp-account-raplyto', 'member/account/Raplytocard.vue'],
  // 售后
  ['.mp-return-list', 'member/return/List.vue'],
  ['.return-content', 'member/return/List.vue'],
  ['.mp-return-info', 'member/return/Info.vue'],
  ['.mp-return-log', 'member/return/Log.vue'],
  ['.mp-return-loginfo', 'member/return/LogInfo.vue'],
  // 拼团
  ['.mp-pin-list', 'member/pin/List.vue'],
  ['.mp-pin-group', 'member/pin/Group.vue'],
  ['.mp-group-info', 'member/group/Info.vue'],
  // 评价
  ['.mp-comment-list', 'member/comment/List.vue'],
  ['.mp-comment-info', 'member/comment/Info.vue'],
  ['.mp-comment-card', 'components/member/comment/CommentList.vue'],
  // 发票
  ['.mp-orderinvoice-list', 'member/orderInvoice/List.vue'],
  ['.mp-orderinvoice-info', 'member/orderInvoice/Info.vue'],
  ['.mp-userinvoice-info', 'member/userInvoice/Info.vue'],
  ['.mp-userinvoice-confirm', 'member/userInvoice/confirmInfo.vue'],
  // 发票弹窗（列表页其实用的是共享组件，不是 Info.vue）
  ['.mp-invoice-dialog', 'components/invoice/InvoiceDialog.vue'],
  ['.mp-invoice-dialog .audit-status', 'components/invoice/InvoiceDialog.vue'],
  ['.mp-invoice-dialog .invoice-img', 'components/invoice/InvoiceDialog.vue'],
  // 拼团 / 认款
  ['.mp-pin-group', 'member/pin/Group.vue'],
  ['.group-info-body', 'member/pin/Group.vue'],
  ['.mp-claim-info', 'member/order/claimInfo.vue'],
  ['.claim-summary', 'member/order/claimInfo.vue'],
  ['.voucher-list', 'member/order/claimInfo.vue'],
  // 消息 / 安全 / 足迹
  ['.mp-usermessage-list', 'member/userMessage/List.vue'],
  ['.mp-security-info', 'member/security/Info.vue'],
  ['.mp-security-dialog', 'member/security/dialogInfo.vue'],
  ['.mp-browsing-list', 'member/browsingHistory/List.vue'],
  // 共享外壳
  ['.info-row', 'assets/css/member/member.less'],
  ['.title-or-tabs', 'assets/css/member/member.less'],
  ['.tig-tabs', 'assets/css/member/member.less'],
  ['.no-result', 'assets/css/member/member.less'],
  ['.el-page', 'assets/css/member/member.less'],
];

const missing = [];
const found = [];
for (const [selector, from] of EXPECT) {
  if (all.includes(selector)) found.push([selector, from]);
  else missing.push([selector, from]);
}

console.log(`扫描 ${cssFiles.length} 个产物 CSS（${(all.length / 1024).toFixed(0)} KB）`);
console.log(`旧版选择器：命中 ${found.length} / ${EXPECT.length}`);
for (const [selector, from] of found) console.log(`  ok   ${selector.padEnd(28)} ← ${from}`);
if (missing.length) {
  console.log('缺失：');
  for (const [selector, from] of missing) console.log(`  MISS ${selector.padEnd(28)} ← ${from}`);
}

// 哪个文件承载这些样式（方便定位）
const probe = '.mp-address-list';
const owner = cssFiles.find((f, i) => bundle[i].includes(probe));
if (owner) console.log(`样式承载文件（含 ${probe}）：${owner}`);

process.exit(missing.length ? 1 : 0);
