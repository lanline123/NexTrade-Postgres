/**
 * 装修模块标题双端一致性回归门禁（只读，不写任何线上数据）。
 *
 * 用法：
 *   NT_ADMIN_TOKEN=<token>        node scripts/verify-title-parity.cjs
 *   NT_ADMIN_USER=u NT_ADMIN_PASSWORD=p  node scripts/verify-title-parity.cjs
 *
 * 可选环境变量：
 *   NT_ADMIN_BASE  默认 https://admin.recy.cc
 *   NT_PC_BASE     默认 https://battery.recy.cc
 *   NT_PAGE_ID     默认 10023
 *   NT_TITLE_BASELINE 默认 18（配置型标题基准字号 px）
 *
 * 覆盖：
 *   A. 编辑器画布：配置型标题统一基准（18px / 行高 32px）；固定标题 24px/700、
 *      秒杀行高 30px；优惠券 18px/500/26px
 *   B. PC 前台：top-bar 18px、固定标题 24px/700、优惠券 18px/500
 *   C. 两端同名字号集合一致
 *
 * 退出码：0 通过 / 1 有失败 / 2 环境缺失（无法登录或站点不可达）
 */
const { chromium } = require('playwright-core');

const ADMIN_BASE = process.env.NT_ADMIN_BASE || 'https://admin.recy.cc';
const PC_BASE = process.env.NT_PC_BASE || 'https://battery.recy.cc';
const PAGE_ID = process.env.NT_PAGE_ID || '10023';
const BASELINE = Number(process.env.NT_TITLE_BASELINE || 18);

async function getToken() {
  if (process.env.NT_ADMIN_TOKEN) return process.env.NT_ADMIN_TOKEN.trim();
  const user = process.env.NT_ADMIN_USER;
  const password = process.env.NT_ADMIN_PASSWORD;
  if (!user || !password) return null;
  const csrfRes = await fetch(`${ADMIN_BASE}/adminapi/common/csrf/create`);
  const cookies = (csrfRes.headers.getSetCookie?.() || [])
    .map((line) => line.split(';')[0])
    .join('; ');
  const csrf = (await csrfRes.json()).data;
  if (!csrf) throw new Error('CSRF 获取失败');
  const loginRes = await fetch(`${ADMIN_BASE}/adminapi/login/signin`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      'X-CSRF-Token': csrf,
      ...(cookies ? { Cookie: cookies } : {}),
    },
    body: JSON.stringify({ loginType: 'password', username: user, password, captchaUid: '' }),
  });
  const body = await loginRes.json();
  return (body.data || {}).token || null;
}

const styleOf = (page, selector) =>
  page.evaluate((sel) => {
    const el = document.querySelector(sel);
    if (!el) return null;
    const cs = getComputedStyle(el);
    return {
      size: parseFloat(cs.fontSize),
      weight: Number(cs.fontWeight),
      line: cs.lineHeight === 'normal' ? 'normal' : parseFloat(cs.lineHeight),
      offsetH: el.offsetHeight, // 不受编辑器画布 scale 影响（HANDOVER §11）
    };
  }, selector);

const allSizes = (page, selector) =>
  page.evaluate((sel) =>
    [...document.querySelectorAll(sel)].map((el) => ({
      text: (el.textContent || '').trim().slice(0, 12),
      size: parseFloat(getComputedStyle(el).fontSize),
      line: parseFloat(getComputedStyle(el).lineHeight) || null,
    })), selector);

async function main() {
  const token = await getToken();
  if (!token) {
    console.error('缺少登录态：请设置 NT_ADMIN_TOKEN，或 NT_ADMIN_USER + NT_ADMIN_PASSWORD');
    process.exit(2);
  }

  const browser = await chromium.launch({
    channel: process.env.NT_BROWSER_CHANNEL || 'msedge',
    headless: true,
    args: ['--no-proxy-server'],
  });
  const failures = [];
  const checks = [];
  const record = (name, ok, actual, expected) => {
    checks.push({ name, ok, actual, expected });
    if (!ok) failures.push(`${name}: 期望 ${expected}，实际 ${actual}`);
  };

  try {
    // —— A. 编辑器画布 ——
    const ctx = await browser.newContext({ viewport: { width: 1720, height: 1000 } });
    await ctx.addInitScript((tk) => {
      localStorage.setItem('admin_access_token', tk);
      localStorage.setItem('adminType', 'admin');
    }, token);
    const ed = await ctx.newPage();
    await ed.goto(`${ADMIN_BASE}/?page=pcDecorateEditor&id=${PAGE_ID}`, { waitUntil: 'domcontentloaded' });
    await ed.waitForSelector('.pc-decorate-module', { timeout: 30000 });
    await ed.waitForTimeout(2500);

    const configTitles = await allSizes(ed, '.dec-module-title__text');
    record(
      '编辑器·配置型标题数量 > 0',
      configTitles.length > 0,
      configTitles.length,
      '>= 1',
    );
    // 字号是可调设置：默认值 BASELINE，允许自定义，但必须落在合法区间（12~48 整数）
    const customSizes = configTitles.filter((x) => x.size !== BASELINE);
    record(
      '编辑器·配置型标题字号合法（默认18 / 可自定义）',
      configTitles.length > 0 &&
        configTitles.every((x) => Number.isInteger(x.size) && x.size >= 12 && x.size <= 48),
      [...new Set(configTitles.map((x) => x.size))].join(',') + (customSizes.length ? ` (自定义 ${customSizes.length} 个)` : ' (全默认)'),
      `12~48，缺省 ${BASELINE}px`,
    );
    record(
      '编辑器·配置型标题行高 32px',
      configTitles.length > 0 && configTitles.every((x) => x.line === 32),
      [...new Set(configTitles.map((x) => x.line))].join(','),
      '32px',
    );

    for (const [name, selector] of [
      ['热门分类', '.dec-pc-category-nav__title'],
      ['分类导航3', '.dec-pc-category-nav3__title'],
      ['秒杀', '.pc-seckill__title'],
      ['拼团', '.pc-groupon__title'],
    ]) {
      const st = await styleOf(ed, selector);
      record(`编辑器·${name} 24px/700`, !!st && st.size === 24 && st.weight === 700,
        st ? `${st.size}px/${st.weight}` : '缺失', '24px/700');
    }
    const seckillLine = await styleOf(ed, '.pc-seckill__title');
    record('编辑器·秒杀标题行高 30px', !!seckillLine && seckillLine.line === 30,
      seckillLine ? seckillLine.line : '缺失', '30px');
    const couponEd = await styleOf(ed, '.dec-coupon__header-title');
    record('编辑器·优惠券字重/行高基线（默认18/500/26，字号可自定义）',
      !!couponEd && couponEd.weight === 500 && couponEd.line === 26 &&
        Number.isInteger(couponEd.size) && couponEd.size >= 12 && couponEd.size <= 48,
      couponEd ? `${couponEd.size}px/${couponEd.weight}/${couponEd.line}px` : '缺失',
      '12~48, 500/26');
    await ctx.close();

    // —— B. PC 前台 ——
    const ctx2 = await browser.newContext({ viewport: { width: 1440, height: 1000 } });
    const fe = await ctx2.newPage();
    await fe.goto(`${PC_BASE}/`, { waitUntil: 'domcontentloaded' });
    await fe.waitForSelector('.top-bar .title-text', { timeout: 30000 });
    await fe.waitForTimeout(2500);
    const feTitles = await allSizes(fe, '.top-bar .title-text');
    record('前台·top-bar 标题数量 > 0', feTitles.length > 0, feTitles.length, '>= 1');
    // 前台可能用 DecModuleTitle（推荐商品）渲染，一并纳入按文本匹配
    const feAll = [
      ...feTitles,
      ...(await allSizes(fe, '.dec-module-title__text')),
    ];
    record('前台·配置型标题字号合法（默认18 / 可自定义）',
      feAll.length > 0 && feAll.every((x) => Number.isInteger(x.size) && x.size >= 12 && x.size <= 48),
      [...new Set(feAll.map((x) => x.size))].join(','), `12~48，缺省 ${BASELINE}px`);
    for (const [name, selector] of [
      ['热门分类', '.dec-category-nav-wrapper .seckill-title'],
      ['分类导航3', '.dec-category-nav3__title'],
      ['秒杀', '.module-seckill-goods_ad .seckill-title'],
    ]) {
      const st = await styleOf(fe, selector);
      record(`前台·${name} 24px/700`, !!st && st.size === 24 && st.weight === 700,
        st ? `${st.size}px/${st.weight}` : '缺失', '24px/700');
    }
    const couponFe = await styleOf(fe, '.dec-coupon__header-title');
    record('前台·优惠券字重基线（默认18/500，字号可自定义）',
      !!couponFe && couponFe.weight === 500 &&
        Number.isInteger(couponFe.size) && couponFe.size >= 12 && couponFe.size <= 48,
      couponFe ? `${couponFe.size}px/${couponFe.weight}` : '缺失', '12~48, 500');

    // —— C. 双端按标题文本逐个一致（这才是「调整后 PC 端同步生效」的门禁） ——
    const edMap = new Map(configTitles.map((x) => [x.text, x.size]));
    const feMap = new Map(feAll.map((x) => [x.text, x.size]));
    const common = [...edMap.keys()].filter((k) => feMap.has(k));
    const mismatched = common.filter((k) => edMap.get(k) !== feMap.get(k));
    record('双端·同标题字号一致',
      common.length > 0 && mismatched.length === 0,
      `共同 ${common.length} 个${mismatched.length ? '，不一致: ' + mismatched.map((k) => `${k}(${edMap.get(k)}/${feMap.get(k)})`).join(' ') : ''}`,
      '逐个相同');
    console.log('字号明细（编辑器）：', JSON.stringify(configTitles.map((x) => `${x.text}=${x.size}`)));
    console.log('字号明细（前台）  ：', JSON.stringify(feAll.map((x) => `${x.text}=${x.size}`)));
    await ctx2.close();
  } finally {
    await browser.close();
  }

  console.log('装修标题双端一致性回归：\n');
  console.log('| 断言 | 实际 | 期望 | 结果 |');
  console.log('|---|---|---|---|');
  for (const c of checks) {
    console.log(`| ${c.name} | ${c.actual} | ${c.expected} | ${c.ok ? 'PASS' : 'FAIL'} |`);
  }
  if (failures.length) {
    console.log(`\n${failures.length} 项失败：`);
    failures.forEach((f) => console.log('  - ' + f));
    process.exit(1);
  }
  console.log(`\n全部 ${checks.length} 项通过。`);
}

main().catch((err) => {
  console.error('回归脚本异常：', err && err.message);
  process.exit(2);
});
