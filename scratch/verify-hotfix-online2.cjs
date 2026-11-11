const { chromium } = require('D:/tigshop/NexTrade-Postgres/admin-semi/node_modules/playwright-core');

(async () => {
  const browser = await chromium.connectOverCDP('http://127.0.0.1:9222');
  const ctx = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const page = await ctx.newPage();
  const cdp = await ctx.newCDPSession(page);
  await cdp.send('Network.enable');
  await cdp.send('Network.setCacheDisabled', { cacheDisabled: true });
  await cdp.send('Network.clearBrowserCache');
  await page.goto('https://battery.recy.cc/?hotfix=' + Date.now(), { waitUntil: 'domcontentloaded', timeout: 90000 });
  try {
    await page.waitForSelector('.dec-category-nav-wrapper .seckill_list .proditem', { timeout: 20000 });
  } catch (e) {
    console.log('WAIT FAILED:', String(e).slice(0, 160));
  }
  await page.waitForTimeout(1200);

  const out = await page.evaluate(() => {
    const w = [...document.querySelectorAll('.dec-category-nav-wrapper')]
      .find((x) => x.querySelector('.seckill_list .proditem'));
    const sheets = [...document.styleSheets].map((s) => (s.href || 'inline').split('/').pop());
    if (!w) return { error: 'category nav not found', modules: document.querySelectorAll('.decorate-renderer__module').length, sheets };
    const prev = w.querySelector('.dec-swiper-nav--prev');
    const next = w.querySelector('.dec-swiper-nav--next');
    const img = w.querySelector('.seckill-item-image');
    const mod = w.getBoundingClientRect();
    const p = prev?.getBoundingClientRect();
    const n = next?.getBoundingClientRect();
    const i = img?.getBoundingClientRect();
    const cs = prev ? getComputedStyle(prev) : null;
    const center = (r) => (r ? +(r.top + r.height / 2).toFixed(1) : null);
    return {
      sheetsWithDecSwiper: sheets.filter((s) => s && s.includes('.css')),
      moduleLeft: +mod.left.toFixed(1),
      moduleRight: +mod.right.toFixed(1),
      prevLeft: p ? +p.left.toFixed(1) : null,
      nextRight: n ? +n.right.toFixed(1) : null,
      arrowCenter: center(p),
      imageCenter: center(i),
      delta: p && i ? +(center(p) - center(i)).toFixed(1) : null,
      top: cs?.top,
      backgroundColor: cs?.backgroundColor,
      zIndex: cs?.zIndex,
    };
  });
  console.log(JSON.stringify(out, null, 2));

  const target = await page.$('.dec-category-nav-wrapper:has(.seckill_list .proditem)');
  if (target) {
    await target.scrollIntoViewIfNeeded();
    await page.waitForTimeout(400);
    await target.screenshot({ path: 'D:/tigshop/NexTrade-Postgres/scratch/category-nav-online-after.png' });
    console.log('SCREENSHOT: scratch/category-nav-online-after.png');
  }
  await ctx.close();
  await browser.close();
})().catch((e) => { console.error(e); process.exit(1); });
