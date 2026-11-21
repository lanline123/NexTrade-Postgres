'use strict';
/**
 * 授权判定核心（与 NexTrade 客户端线的共享契约，字段/算法逐字约定见 README「checkin 契约」）。
 *
 * 判定规则（用户口径）：
 * - 商业授权：只可绑定 1 个固定域名，上报域名（adminHost + siteDomains）任一与之全等 → authorized/commercial。
 * - 企业授权：绑定 1 个泛域名，上报域名为其子域名时：已登记 → authorized/enterprise；
 *   未登记且额度未满 → 首次命中自动登记占用额度 → authorized/enterprise；
 *   额度已满 → unauthorized（reason=quota_full，管理端标红「额度已满」）。
 * - 授权到期（expiresAt > 0 且 <= now）→ unauthorized，tier 保留回显，响应带 expired: true。
 * - 停用（status=disabled）的授权不参与判定；都不命中 → unauthorized（reason=no_match）。
 * - 精确（商业）优先于泛域名（企业）；未过期授权优先于已过期授权。
 *
 * 响应签名：signKey = SHA256("auth-semi-sign-v1:" + instanceKey)，
 * signature = HMAC-SHA256(signKey, [status, tier, expiresAt, subdomainLimit, subdomainsUsed, checkedAt].join("|")) 的 hex；
 * 空值（null）以空串参与拼接。客户端以同一算法验签，防伪造响应。
 */
const crypto = require('node:crypto');

/** 归一化主机名：去协议/路径/端口/尾点/泛前缀，转小写。 */
function normalizeHost(input) {
  if (input === undefined || input === null) return '';
  let s = String(input).trim().toLowerCase();
  if (!s) return '';
  s = s.replace(/^[a-z][a-z0-9+.-]*:\/\//, '');
  s = s.split('/')[0].split('?')[0].split('#')[0];
  s = s.replace(/:\d+$/, '');
  s = s.replace(/^\*\./, '');
  s = s.replace(/\.$/, '');
  return s;
}

const DOMAIN_RE = /^[a-z0-9]([a-z0-9-]*[a-z0-9])?(\.[a-z0-9]([a-z0-9-]*[a-z0-9])?)+$/;
function isValidDomain(d) {
  return typeof d === 'string' && d.length <= 253 && DOMAIN_RE.test(d);
}

/** d 是否为 w 的真子域名（至少多一级标签）。 */
function isSubdomainOf(d, w) {
  return d.length > w.length + 1 && d.endsWith(`.${w}`);
}

function isExpired(lic, nowMs) {
  return lic.expiresAt > 0 && lic.expiresAt <= nowMs;
}

/**
 * 对一次上报做判定。candidates 为归一化后的候选域名数组（adminHost 在前）。
 * 企业版首次命中新子域名时直接在本函数内登记（调用方负责在 mutate 中落盘）。
 * 返回 { status, tier, licenseId, expiresAt, subdomainLimit, subdomainsUsed, reason, expired?, quotaFull? }
 */
function decide(data, candidates, nowMs) {
  const active = data.licenses.filter((l) => l.status === 'active');
  let expiredMatch = null;

  // 第一遍：商业版固定域名精确命中
  for (const lic of active) {
    if (lic.tier !== 'commercial') continue;
    if (!candidates.includes(lic.domain)) continue;
    if (isExpired(lic, nowMs)) {
      expiredMatch = expiredMatch || lic;
      continue;
    }
    return {
      status: 'authorized', tier: 'commercial', licenseId: lic.id,
      expiresAt: lic.expiresAt || null, subdomainLimit: null, subdomainsUsed: null,
      reason: 'commercial_match',
    };
  }

  // 第二遍：企业版泛域名子域名命中
  for (const lic of active) {
    if (lic.tier !== 'enterprise') continue;
    const hit = candidates.find((d) => isSubdomainOf(d, lic.wildcardDomain));
    if (!hit) continue;
    if (isExpired(lic, nowMs)) {
      expiredMatch = expiredMatch || lic;
      continue;
    }
    if (lic.subdomains.includes(hit)) {
      return {
        status: 'authorized', tier: 'enterprise', licenseId: lic.id,
        expiresAt: lic.expiresAt || null,
        subdomainLimit: lic.subdomainLimit, subdomainsUsed: lic.subdomains.length,
        reason: 'enterprise_registered',
      };
    }
    if (lic.subdomains.length < lic.subdomainLimit) {
      lic.subdomains.push(hit);
      lic.updatedAt = nowMs;
      return {
        status: 'authorized', tier: 'enterprise', licenseId: lic.id,
        expiresAt: lic.expiresAt || null,
        subdomainLimit: lic.subdomainLimit, subdomainsUsed: lic.subdomains.length,
        reason: 'enterprise_auto_registered',
      };
    }
    return {
      status: 'unauthorized', tier: 'enterprise', licenseId: lic.id,
      expiresAt: lic.expiresAt || null,
      subdomainLimit: lic.subdomainLimit, subdomainsUsed: lic.subdomains.length,
      reason: 'quota_full', quotaFull: true,
    };
  }

  if (expiredMatch) {
    const lic = expiredMatch;
    return {
      status: 'unauthorized', tier: lic.tier, licenseId: lic.id,
      expiresAt: lic.expiresAt || null,
      subdomainLimit: lic.tier === 'enterprise' ? lic.subdomainLimit : null,
      subdomainsUsed: lic.tier === 'enterprise' ? lic.subdomains.length : null,
      reason: 'expired', expired: true,
    };
  }

  return {
    status: 'unauthorized', tier: null, licenseId: null,
    expiresAt: null, subdomainLimit: null, subdomainsUsed: null,
    reason: 'no_match',
  };
}

function deriveSignKey(instanceKey) {
  return crypto.createHash('sha256').update(`auth-semi-sign-v1:${String(instanceKey)}`, 'utf8').digest();
}

function signResponse(instanceKey, r) {
  const payload = [
    r.status,
    r.tier || '',
    r.expiresAt === null || r.expiresAt === undefined ? '' : String(r.expiresAt),
    r.subdomainLimit === null || r.subdomainLimit === undefined ? '' : String(r.subdomainLimit),
    r.subdomainsUsed === null || r.subdomainsUsed === undefined ? '' : String(r.subdomainsUsed),
    String(r.checkedAt),
  ].join('|');
  return crypto.createHmac('sha256', deriveSignKey(instanceKey)).update(payload, 'utf8').digest('hex');
}

module.exports = { normalizeHost, isValidDomain, isSubdomainOf, isExpired, decide, signResponse };
