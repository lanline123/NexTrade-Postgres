'use strict';
/**
 * NexTrade 授权中心（auth.recy.cc）——零依赖 Node 服务（仅用 node: 标准库）。
 * 与 pc-semi / admin-semi 并列的独立应用：独立进程（systemd nextrade-auth）、
 * 独立站点、独立数据文件，互不引用。
 *
 * 面向对象：
 * - NexTrade 部署实例：POST /api/instances/checkin 上报并取授权判定（契约见 README）。
 * - 运营（平台方）：/admin.html 管理端——授权 CRUD、企业版子域名额度调整、实例上报聚合。
 *
 * 配置（环境变量或项目根 .env 文件，环境变量优先）：
 *   PORT                监听端口，默认 3011（只绑 127.0.0.1，由 nginx 反代对外）
 *   HOST                监听地址，默认 127.0.0.1
 *   AUTH_ADMIN_PASSWORD 管理端密码（未设置则管理端禁用，checkin 照常）
 *   AUTH_SESSION_SECRET 会话签名密钥（未设置自动生成并存 data/.session-secret）
 *   AUTH_DATA_FILE      数据文件路径，默认 <项目>/data/db.json
 */
const http = require('node:http');
const fs = require('node:fs');
const path = require('node:path');
const { Store, CHECKIN_LOG_CAP } = require('./store');
const { Auth } = require('./auth');
const lic = require('./license');

const ROOT = path.resolve(__dirname, '..');
const PUBLIC_DIR = path.join(ROOT, 'public');

/* ---------------- 配置 ---------------- */
function loadDotEnv() {
  try {
    const raw = fs.readFileSync(path.join(ROOT, '.env'), 'utf8');
    for (const line of raw.split(/\r?\n/)) {
      const m = line.match(/^\s*([A-Za-z_][A-Za-z0-9_]*)\s*=\s*(.*)\s*$/);
      if (m && process.env[m[1]] === undefined) {
        process.env[m[1]] = m[2].replace(/^["']|["']$/g, '');
      }
    }
  } catch (_) { /* 无 .env 文件时忽略 */ }
}
loadDotEnv();

const PORT = Number(process.env.PORT || 3011);
const HOST = process.env.HOST || '127.0.0.1';
const DATA_FILE = process.env.AUTH_DATA_FILE || path.join(ROOT, 'data', 'db.json');

const store = new Store(DATA_FILE);
const auth = new Auth({
  password: process.env.AUTH_ADMIN_PASSWORD || '',
  dataDir: path.dirname(DATA_FILE),
  envSecret: process.env.AUTH_SESSION_SECRET || '',
});

/* ---------------- 工具 ---------------- */
function sendJson(res, status, obj) {
  const body = JSON.stringify(obj);
  res.writeHead(status, {
    'Content-Type': 'application/json; charset=utf-8',
    'Content-Length': Buffer.byteLength(body),
  });
  res.end(body);
}
const ok = (res, data) => sendJson(res, 200, { code: 0, msg: 'success', data });
const fail = (res, status, msg) => sendJson(res, status, { code: status, msg, data: null });

function readBody(req) {
  return new Promise((resolve, reject) => {
    const chunks = [];
    let size = 0;
    req.on('data', (c) => {
      size += c.length;
      if (size > 2 * 1024 * 1024) {
        reject(new Error('请求体过大'));
        req.destroy();
        return;
      }
      chunks.push(c);
    });
    req.on('end', () => {
      const text = Buffer.concat(chunks).toString('utf8').trim();
      if (!text) return resolve({});
      try {
        resolve(JSON.parse(text));
      } catch (_) {
        reject(new Error('请求体不是合法 JSON'));
      }
    });
    req.on('error', reject);
  });
}

const now = () => Date.now();

function licenseView(l) {
  return {
    id: l.id,
    customerName: l.customerName,
    note: l.note,
    tier: l.tier,
    domain: l.domain,
    wildcardDomain: l.wildcardDomain,
    subdomainLimit: l.subdomainLimit,
    subdomains: [...l.subdomains],
    subdomainsUsed: l.tier === 'enterprise' ? l.subdomains.length : null,
    status: l.status,
    expiresAt: l.expiresAt,
    expired: lic.isExpired(l, now()),
    createdAt: l.createdAt,
    updatedAt: l.updatedAt,
  };
}

function instanceView(key, inst) {
  return {
    instanceKey: key,
    adminHost: inst.adminHost,
    siteDomains: inst.siteDomains,
    appVersion: inst.appVersion,
    firstSeenAt: inst.firstSeenAt,
    lastSeenAt: inst.lastSeenAt,
    lastStatus: inst.lastStatus,
    lastTier: inst.lastTier,
    lastReason: inst.lastReason,
    quotaFull: inst.lastReason === 'quota_full',
    checkinCount: inst.checkinCount,
  };
}

/* ---------------- 实例 checkin（对外契约） ---------------- */
async function handleCheckin(req, res) {
  const body = await readBody(req);
  const instanceKey = String(body.instanceKey || '').trim();
  if (!instanceKey || instanceKey.length > 128) {
    return sendJson(res, 400, { status: 'unauthorized', tier: null, error: 'instanceKey 必填且不超过 128 字符' });
  }
  const adminHost = lic.normalizeHost(body.adminHost || '');
  const rawDomains = Array.isArray(body.siteDomains) ? body.siteDomains : [];
  const siteDomains = [...new Set(rawDomains.map((d) => lic.normalizeHost(d)).filter(Boolean))].slice(0, 50);
  const appVersion = String(body.appVersion || '').slice(0, 64);
  const candidates = [...new Set([adminHost, ...siteDomains].filter(Boolean))];
  const checkedAt = now();

  const decision = store.mutate((d) => {
    const dec = lic.decide(d, candidates, checkedAt);
    const prev = d.instances[instanceKey];
    d.instances[instanceKey] = {
      instanceKey,
      adminHost,
      siteDomains,
      appVersion,
      firstSeenAt: prev ? prev.firstSeenAt : checkedAt,
      lastSeenAt: checkedAt,
      lastStatus: dec.status,
      lastTier: dec.tier,
      lastReason: dec.reason,
      checkinCount: (prev ? prev.checkinCount : 0) + 1,
    };
    d.checkins.push({
      id: store.nextId('checkin'),
      instanceKey, adminHost, siteDomains, appVersion,
      status: dec.status, tier: dec.tier, reason: dec.reason, at: checkedAt,
    });
    if (d.checkins.length > CHECKIN_LOG_CAP) {
      d.checkins.splice(0, d.checkins.length - CHECKIN_LOG_CAP);
    }
    return dec;
  });

  const resp = {
    status: decision.status,
    tier: decision.tier,
    expiresAt: decision.expiresAt,
    subdomainLimit: decision.subdomainLimit,
    subdomainsUsed: decision.subdomainsUsed,
    checkedAt,
  };
  if (decision.expired) resp.expired = true;
  resp.signature = lic.signResponse(instanceKey, resp);
  return sendJson(res, 200, resp);
}

/* ---------------- 授权入参校验 ---------------- */
function parseLicenseInput(body, base) {
  const tier = body.tier !== undefined ? String(body.tier) : base.tier;
  if (tier !== 'commercial' && tier !== 'enterprise') return { error: 'tier 必须是 commercial 或 enterprise' };

  const out = { ...base, tier };

  if (body.customerName !== undefined) out.customerName = String(body.customerName).trim().slice(0, 128);
  if (body.note !== undefined) out.note = String(body.note).trim().slice(0, 512);
  if (body.status !== undefined) {
    if (body.status !== 'active' && body.status !== 'disabled') return { error: 'status 必须是 active 或 disabled' };
    out.status = body.status;
  }
  if (body.expiresAt !== undefined) {
    if (body.expiresAt === null || body.expiresAt === 0 || body.expiresAt === '') {
      out.expiresAt = 0;
    } else {
      const t = Number(body.expiresAt);
      if (!Number.isFinite(t) || t < 0) return { error: 'expiresAt 必须是 ms 时间戳（0/空 = 永不过期）' };
      out.expiresAt = Math.floor(t);
    }
  }

  if (tier === 'commercial') {
    const domain = body.domain !== undefined ? lic.normalizeHost(body.domain) : base.domain;
    if (!lic.isValidDomain(domain || '')) return { error: '商业授权必须绑定一个合法的固定域名' };
    out.domain = domain;
    out.wildcardDomain = null;
    out.subdomainLimit = null;
    out.subdomains = [];
  } else {
    const wildcardDomain = body.wildcardDomain !== undefined ? lic.normalizeHost(body.wildcardDomain) : base.wildcardDomain;
    if (!lic.isValidDomain(wildcardDomain || '')) return { error: '企业授权必须绑定一个合法的泛域名（如 example.com）' };
    out.wildcardDomain = wildcardDomain;
    out.domain = null;
    if (body.subdomainLimit !== undefined) {
      const n = Number(body.subdomainLimit);
      if (!Number.isInteger(n) || n < 0 || n > 10000) return { error: 'subdomainLimit 必须是非负整数' };
      out.subdomainLimit = n;
    } else if (out.subdomainLimit === null || out.subdomainLimit === undefined) {
      out.subdomainLimit = 5;
    }
    if (body.subdomains !== undefined) {
      if (!Array.isArray(body.subdomains)) return { error: 'subdomains 必须是数组' };
      const subs = [...new Set(body.subdomains.map((s) => lic.normalizeHost(s)).filter(Boolean))];
      const bad = subs.find((s) => !lic.isSubdomainOf(s, wildcardDomain));
      if (bad) return { error: `子域名 ${bad} 不在泛域名 ${wildcardDomain} 之下` };
      out.subdomains = subs;
    } else if (!Array.isArray(out.subdomains)) {
      out.subdomains = [];
    }
  }
  return { value: out };
}

/* ---------------- 管理 API ---------------- */
async function handleAdminApi(req, res, url) {
  const p = url.pathname;

  if (req.method === 'POST' && p === '/api/admin/login') {
    if (!auth.enabled) return fail(res, 503, '管理端未配置密码（AUTH_ADMIN_PASSWORD），请联系部署方');
    const body = await readBody(req);
    if (!auth.checkPassword(body.password)) return fail(res, 401, '密码不正确');
    res.setHeader('Set-Cookie', auth.issueCookie());
    return ok(res, { authed: true });
  }
  if (req.method === 'POST' && p === '/api/admin/logout') {
    res.setHeader('Set-Cookie', auth.clearCookie());
    return ok(res, { authed: false });
  }
  if (req.method === 'GET' && p === '/api/admin/me') {
    return ok(res, { enabled: auth.enabled, authed: auth.enabled && auth.isAuthed(req) });
  }

  if (!auth.enabled) return fail(res, 503, '管理端未配置密码（AUTH_ADMIN_PASSWORD）');
  if (!auth.isAuthed(req)) return fail(res, 401, '未登录');

  /* 授权列表 */
  if (req.method === 'GET' && p === '/api/admin/licenses') {
    const list = [...store.data.licenses].sort((a, b) => a.id - b.id);
    return ok(res, list.map(licenseView));
  }

  /* 新建授权 */
  if (req.method === 'POST' && p === '/api/admin/licenses') {
    const body = await readBody(req);
    const base = {
      customerName: '', note: '', tier: '', domain: null, wildcardDomain: null,
      subdomainLimit: null, subdomains: [], status: 'active', expiresAt: 0,
    };
    const parsed = parseLicenseInput(body, base);
    if (parsed.error) return fail(res, 400, parsed.error);
    const created = store.mutate((d) => {
      const l = { ...parsed.value, id: store.nextId('license'), createdAt: now(), updatedAt: now() };
      d.licenses.push(l);
      return l;
    });
    return ok(res, licenseView(created));
  }

  /* 企业版额度调整（续费/加购：delta 可正可负，结果不低于已用数与 0） */
  const adjustMatch = p.match(/^\/api\/admin\/licenses\/(\d+)\/adjust-limit$/);
  if (adjustMatch && req.method === 'POST') {
    const id = Number(adjustMatch[1]);
    const body = await readBody(req);
    const delta = Number(body.delta);
    if (!Number.isInteger(delta) || delta === 0) return fail(res, 400, 'delta 必须是非零整数');
    const updated = store.mutate((d) => {
      const l = d.licenses.find((x) => x.id === id);
      if (!l || l.tier !== 'enterprise') return null;
      l.subdomainLimit = Math.max(l.subdomains.length, 0, l.subdomainLimit + delta);
      l.updatedAt = now();
      return l;
    });
    if (!updated) return fail(res, 404, '授权不存在或非企业版');
    return ok(res, licenseView(updated));
  }

  /* 释放企业版已登记子域名 */
  const subMatch = p.match(/^\/api\/admin\/licenses\/(\d+)\/subdomains\/(.+)$/);
  if (subMatch && req.method === 'DELETE') {
    const id = Number(subMatch[1]);
    const sub = lic.normalizeHost(decodeURIComponent(subMatch[2]));
    const updated = store.mutate((d) => {
      const l = d.licenses.find((x) => x.id === id);
      if (!l) return null;
      l.subdomains = l.subdomains.filter((s) => s !== sub);
      l.updatedAt = now();
      return l;
    });
    if (!updated) return fail(res, 404, '授权不存在');
    return ok(res, licenseView(updated));
  }

  /* 编辑 / 删除授权 */
  const licMatch = p.match(/^\/api\/admin\/licenses\/(\d+)$/);
  if (licMatch && req.method === 'PUT') {
    const id = Number(licMatch[1]);
    const body = await readBody(req);
    const existing = store.data.licenses.find((x) => x.id === id);
    if (!existing) return fail(res, 404, '授权不存在');
    const parsed = parseLicenseInput(body, { ...existing });
    if (parsed.error) return fail(res, 400, parsed.error);
    const updated = store.mutate((d) => {
      const l = d.licenses.find((x) => x.id === id);
      Object.assign(l, parsed.value, { id: l.id, createdAt: l.createdAt, updatedAt: now() });
      return l;
    });
    return ok(res, licenseView(updated));
  }
  if (licMatch && req.method === 'DELETE') {
    const id = Number(licMatch[1]);
    store.mutate((d) => {
      d.licenses = d.licenses.filter((x) => x.id !== id);
    });
    return ok(res, { deleted: true });
  }

  /* 实例上报聚合 */
  if (req.method === 'GET' && p === '/api/admin/instances') {
    const list = Object.entries(store.data.instances)
      .map(([key, inst]) => instanceView(key, inst))
      .sort((a, b) => b.lastSeenAt - a.lastSeenAt);
    return ok(res, list);
  }
  const instMatch = p.match(/^\/api\/admin\/instances\/(.+)$/);
  if (instMatch && req.method === 'DELETE') {
    const key = decodeURIComponent(instMatch[1]);
    store.mutate((d) => {
      delete d.instances[key];
      d.checkins = d.checkins.filter((c) => c.instanceKey !== key);
    });
    return ok(res, { deleted: true });
  }

  /* 最近上报记录 */
  if (req.method === 'GET' && p === '/api/admin/checkins') {
    const limit = Math.min(200, Math.max(1, Number(url.searchParams.get('limit') || 50)));
    return ok(res, [...store.data.checkins].slice(-limit).reverse());
  }

  return fail(res, 404, '接口不存在');
}

/* ---------------- 静态文件 ---------------- */
const MIME = {
  '.html': 'text/html; charset=utf-8',
  '.css': 'text/css; charset=utf-8',
  '.js': 'text/javascript; charset=utf-8',
  '.json': 'application/json; charset=utf-8',
  '.png': 'image/png',
  '.jpg': 'image/jpeg',
  '.svg': 'image/svg+xml',
  '.ico': 'image/x-icon',
};

function serveStatic(req, res, url) {
  let pathname = decodeURIComponent(url.pathname);
  if (pathname === '/') pathname = '/index.html';
  if (pathname === '/admin') pathname = '/admin.html';
  const file = path.join(PUBLIC_DIR, pathname);
  if (!file.startsWith(PUBLIC_DIR)) return fail(res, 403, '禁止访问');
  fs.readFile(file, (err, content) => {
    if (err) return fail(res, 404, '页面不存在');
    res.writeHead(200, {
      'Content-Type': MIME[path.extname(file)] || 'application/octet-stream',
      'Cache-Control': 'no-cache',
    });
    res.end(content);
  });
}

/* ---------------- 主服务 ---------------- */
const server = http.createServer(async (req, res) => {
  const url = new URL(req.url, `http://${req.headers.host || 'localhost'}`);
  try {
    if (url.pathname.startsWith('/api/')) {
      res.setHeader('Access-Control-Allow-Origin', '*');
      res.setHeader('Access-Control-Allow-Methods', 'GET, POST, PUT, DELETE, OPTIONS');
      res.setHeader('Access-Control-Allow-Headers', 'Content-Type');
      if (req.method === 'OPTIONS') {
        res.writeHead(204);
        return res.end();
      }
      if (req.method === 'POST' && url.pathname === '/api/instances/checkin') {
        return await handleCheckin(req, res);
      }
      if (url.pathname.startsWith('/api/admin/')) {
        return await handleAdminApi(req, res, url);
      }
      return fail(res, 404, '接口不存在');
    }
    return serveStatic(req, res, url);
  } catch (e) {
    return fail(res, 500, `服务内部错误：${e.message}`);
  }
});

server.listen(PORT, HOST, () => {
  console.log(`[auth-platform] listening on http://${HOST}:${PORT}`);
  console.log(`[auth-platform] data file: ${DATA_FILE}`);
  console.log(`[auth-platform] admin: ${auth.enabled ? 'enabled' : 'DISABLED (set AUTH_ADMIN_PASSWORD)'}`);
});
