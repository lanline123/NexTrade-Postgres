#!/usr/bin/env node
/**
 * 抓 dev server 的 /api/common/config/base 存成文件，并直接报告 memberWithdrawalEnabled。
 * 用法：node scripts/_fetch-cfg.mjs [outFile] [url]
 */
import fs from 'node:fs';

const out = process.argv[2] || '.tmp-r27/cfg.json';
const url = process.argv[3] || 'http://127.0.0.1:3004/api/common/config/base';
const response = await fetch(url);
const text = await response.text();
fs.writeFileSync(out, text);
console.log(`GET ${url} -> ${response.status}, ${text.length} bytes -> ${out}`);

let json = null;
try {
  json = JSON.parse(text);
} catch (error) {
  console.log(`not JSON: ${String(error.message).slice(0, 120)}`);
  console.log(`first 200 chars: ${text.slice(0, 200)}`);
  process.exit(0);
}

const data = json?.data || json || {};
const keys = Object.keys(data);
const withdraw = keys.filter((key) => /withdraw/i.test(key));
console.log(`data keys: ${keys.length}; withdraw-ish: ${withdraw.length ? withdraw.join(', ') : '(none)'}`);
for (const key of withdraw) {
  const value = data[key];
  console.log(`  ${key} = ${JSON.stringify(value)} (typeof ${typeof value})`);
}
const value = data.memberWithdrawalEnabled;
console.log(`memberWithdrawalEnabled = ${JSON.stringify(value)} → Number()===1: ${Number(value) === 1}`);
