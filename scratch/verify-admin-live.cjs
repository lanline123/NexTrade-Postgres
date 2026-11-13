// 只读验收：admin.recy.cc 能否正常打开、登录、进装修编辑器（沙箱需走代理）
const { chromium } = require('playwright-core');

const PROXY = { server: 'http://172.16.10.254:20705' };
const UA = 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36';

(async () => {
  const browser = await chromium.launch({ headless: true, proxy: PROXY, args: ['--no-sandbox', '--disable-dev-shm-usage'] });
  const ctx = await browser.newContext({ viewport: { width: 1560, height: 940 }, userAgent: UA });
  const page = await ctx.newPage();
  const bad = [], errs = [];
  page.on('response', (r) => { const u = r.url(); if (r.status() >= 400 && /\.(js|css|woff2?|png|svg)(\?|$)/.test(u)) bad.push(r.status() + ' ' + u.slice(0, 110)); });
  page.on('console', (m) => { if (m.type() === 'error') errs.push(m.text().slice(0, 150)); });

  const out = {};
  await page.goto('https://admin.recy.cc/', { waitUntil: 'domcontentloaded', timeout: 60000 });
  await page.waitForTimeout(4500);
  out.login_page_text = (await page.locator('body').innerText()).replace(/\s+/g, ' ').slice(0, 160);
  out.inputs = await page.locator('input').count();
  out.password_inputs = await page.locator('input[type=password]').count();
  await page.screenshot({ path: '/tmp/adm-login.png' });

  if (out.password_inputs > 0) {
    const textInput = page.locator('input:not([type=password])').first();
    await textInput.fill('admin');
    await page.locator('input[type=password]').first().fill('admin123');
    const btn = page.locator('button:has-text("登录"), button[type=submit]').first();
    await btn.click().catch(() => {});
    await page.waitForTimeout(6000);
    out.after_login_url = page.url();
    out.after_login_text = (await page.locator('body').innerText()).replace(/\s+/g, ' ').slice(0, 200);
    out.token_in_ls = await page.evaluate(() => Boolean(localStorage.getItem('admin_access_token')));
    await page.screenshot({ path: '/tmp/adm-after-login.png' });

    await page.goto('https://admin.recy.cc/?page=pcDecorateEditor&id=10023', { waitUntil: 'domcontentloaded', timeout: 60000 }).catch(() => {});
    await page.waitForTimeout(9000);
    out.editor_url = page.url();
    out.editor_modules = await page.locator('.pc-decorate-module').count();
    out.editor_text = (await page.locator('body').innerText()).replace(/\s+/g, ' ').slice(0, 200);
    await page.screenshot({ path: '/tmp/adm-editor.png' });
  }

  out.bad_resources = bad.slice(0, 8);
  out.console_errors = errs.slice(0, 6);
  console.log(JSON.stringify(out, null, 1));
  await browser.close();
})().catch((e) => { console.error('FAILED', e.message); process.exit(1); });
