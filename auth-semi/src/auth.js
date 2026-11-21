'use strict';
/**
 * 管理端单密码登录 + 签名 Cookie 会话（零依赖）。
 * - 密码来自环境变量 AUTH_ADMIN_PASSWORD（部署时服务器侧随机生成，不入库不入代码）。
 * - 会话 Cookie：base64url({exp}) + '.' + HMAC-SHA256 签名；
 *   密钥来自 AUTH_SESSION_SECRET，未设置时生成随机密钥并存 data/.session-secret
 *   （权限 0600），保证重启后会话不失效。
 */
const crypto = require('node:crypto');
const fs = require('node:fs');
const path = require('node:path');

const COOKIE_NAME = 'auth_admin';
const SESSION_TTL_MS = 7 * 24 * 3600 * 1000;

function loadSecret(dataDir, envSecret) {
  if (envSecret) return envSecret;
  const file = path.join(dataDir, '.session-secret');
  try {
    const existing = fs.readFileSync(file, 'utf8').trim();
    if (existing.length >= 32) return existing;
  } catch (_) { /* 首次生成 */ }
  const secret = crypto.randomBytes(32).toString('hex');
  fs.mkdirSync(dataDir, { recursive: true });
  fs.writeFileSync(file, secret, { encoding: 'utf8', mode: 0o600 });
  return secret;
}

function sha256(text) {
  return crypto.createHash('sha256').update(String(text), 'utf8').digest();
}

class Auth {
  constructor({ password, dataDir, envSecret }) {
    this.password = password || '';
    this.secret = loadSecret(dataDir, envSecret);
  }

  get enabled() {
    return this.password.length > 0;
  }

  /** 常量时间比对（先哈希到定长，避免长度泄漏）。 */
  checkPassword(candidate) {
    if (!this.enabled) return false;
    return crypto.timingSafeEqual(sha256(candidate || ''), sha256(this.password));
  }

  sign(payloadB64) {
    return crypto.createHmac('sha256', this.secret).update(payloadB64).digest('base64url');
  }

  issueCookie() {
    const payload = Buffer.from(JSON.stringify({ exp: Date.now() + SESSION_TTL_MS }), 'utf8').toString('base64url');
    const value = `${payload}.${this.sign(payload)}`;
    return `${COOKIE_NAME}=${value}; Path=/; HttpOnly; SameSite=Lax; Max-Age=${Math.floor(SESSION_TTL_MS / 1000)}`;
  }

  clearCookie() {
    return `${COOKIE_NAME}=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0`;
  }

  isAuthed(req) {
    const header = req.headers.cookie || '';
    for (const part of header.split(';')) {
      const idx = part.indexOf('=');
      if (idx < 0) continue;
      if (part.slice(0, idx).trim() !== COOKIE_NAME) continue;
      const value = part.slice(idx + 1).trim();
      const dot = value.lastIndexOf('.');
      if (dot < 0) return false;
      const payload = value.slice(0, dot);
      const sig = value.slice(dot + 1);
      const expected = this.sign(payload);
      const a = Buffer.from(sig);
      const b = Buffer.from(expected);
      if (a.length !== b.length || !crypto.timingSafeEqual(a, b)) return false;
      try {
        const { exp } = JSON.parse(Buffer.from(payload, 'base64url').toString('utf8'));
        return typeof exp === 'number' && exp > Date.now();
      } catch (_) {
        return false;
      }
    }
    return false;
  }
}

module.exports = { Auth, COOKIE_NAME };
