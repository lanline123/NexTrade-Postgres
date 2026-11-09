#!/usr/bin/env node
/** 看 /common/config/base 里 memberWithdrawalEnabled 到底有没有、什么类型、什么值。 */
import fs from 'node:fs';

const file = process.argv[2] || '.tmp-r27/cfg.json';
const raw = fs.readFileSync(file, 'utf8');
const json = JSON.parse(raw);
const data = json?.data || json || {};
const keys = Object.keys(data);
const hit = keys.filter((k) => /withdraw/i.test(k));
const value = data.memberWithdrawalEnabled;
console.log(`file: ${file} (${raw.length} bytes JSON, ${keys.length} keys in data)`);
console.log(`withdraw-ish keys: ${hit.length ? hit.join(', ') : '(none)'}`);
console.log(`memberWithdrawalEnabled = ${JSON.stringify(value)} (typeof ${typeof value})`);
console.log(`Number(...) === 1  →  ${Number(value) === 1}`);
