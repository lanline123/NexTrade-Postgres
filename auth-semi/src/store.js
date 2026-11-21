'use strict';
/**
 * 极简 JSON 文件存储（单进程、同步读写、临时文件 + rename 原子落盘）。
 * 授权量级为运营手工维护（百级以内），实例上报按 instanceKey 聚合，无需数据库。
 *
 * 数据结构：
 * {
 *   "licenses": [{
 *     "id": 1,
 *     "customerName": "客户名", "note": "备注",
 *     "tier": "commercial" | "enterprise",
 *     "domain": "shop.example.com" | null,          // 商业版：唯一固定域名
 *     "wildcardDomain": "example.com" | null,        // 企业版：泛域名基域（子域名判定基）
 *     "subdomainLimit": 5 | null,                    // 企业版：子域名额度
 *     "subdomains": ["a.example.com"],               // 企业版：已登记占用的子域名
 *     "status": "active" | "disabled",
 *     "expiresAt": 0,                                // ms 时间戳；0 = 永不过期
 *     "createdAt": 0, "updatedAt": 0
 *   }],
 *   "instances": {                                   // key = instanceKey
 *     "<key>": { "instanceKey": "...", "adminHost": "...", "siteDomains": ["..."],
 *                "appVersion": "...", "firstSeenAt": 0, "lastSeenAt": 0,
 *                "lastStatus": "authorized" | "unauthorized",
 *                "lastTier": "commercial" | "enterprise" | null,
 *                "lastReason": "commercial_match" | "enterprise_registered" |
 *                              "enterprise_auto_registered" | "quota_full" |
 *                              "expired" | "no_match",
 *                "checkinCount": 0 }
 *   },
 *   "checkins": [{ "id": 1, "instanceKey": "...", "adminHost": "...",
 *                  "siteDomains": ["..."], "appVersion": "...",
 *                  "status": "...", "tier": null, "reason": "...", "at": 0 }],
 *   "seq": { "license": 1, "checkin": 1 }
 * }
 */
const fs = require('node:fs');
const path = require('node:path');

const CHECKIN_LOG_CAP = 1000;

class Store {
  constructor(file) {
    this.file = file;
    this.data = null;
    this.load();
  }

  load() {
    try {
      const raw = fs.readFileSync(this.file, 'utf8');
      const parsed = JSON.parse(raw);
      if (parsed && Array.isArray(parsed.licenses) && parsed.instances && Array.isArray(parsed.checkins) && parsed.seq) {
        this.data = parsed;
        return;
      }
    } catch (_) {
      /* 文件不存在或损坏时按空库启动（损坏文件不覆盖，save 时才写） */
    }
    this.data = { licenses: [], instances: {}, checkins: [], seq: { license: 1, checkin: 1 } };
  }

  save() {
    fs.mkdirSync(path.dirname(this.file), { recursive: true });
    const tmp = `${this.file}.tmp`;
    fs.writeFileSync(tmp, JSON.stringify(this.data, null, 2), 'utf8');
    fs.renameSync(tmp, this.file);
  }

  /** 在内存数据上执行变更并落盘，返回 fn 的返回值。 */
  mutate(fn) {
    const result = fn(this.data);
    this.save();
    return result;
  }

  nextId(kind) {
    const id = this.data.seq[kind];
    this.data.seq[kind] = id + 1;
    return id;
  }
}

module.exports = { Store, CHECKIN_LOG_CAP };
