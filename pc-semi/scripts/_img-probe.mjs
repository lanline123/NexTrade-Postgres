#!/usr/bin/env node
/**
 * 逐个测 checks/round-28/capture.json 里失败的图片地址能不能取到、多快。
 * 用法：node scripts/_img-probe.mjs [capture.json]
 */
import fs from 'node:fs';

const file = process.argv[2] || 'checks/round-28/capture.json';
const raw = JSON.parse(fs.readFileSync(file, 'utf8'));
const failed = (raw?.health?.failed_requests || []).filter((item) => item.type === 'Image');

async function probe(url, method) {
  const started = Date.now();
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), 20000);
  try {
    const response = await fetch(url, { method, signal: controller.signal, redirect: 'follow' });
    const elapsed = Date.now() - started;
    let bytes = '-';
    if (method === 'GET' && response.ok) {
      const buffer = await response.arrayBuffer();
      bytes = String(buffer.byteLength);
    }
    return `${method} ${response.status} ${elapsed}ms ${bytes}B`;
  } catch (error) {
    return `${method} FAIL ${Date.now() - started}ms ${error.name}:${String(error.message).slice(0, 60)}`;
  } finally {
    clearTimeout(timer);
  }
}

for (const item of failed) {
  const head = await probe(item.url, 'HEAD');
  const get = await probe(item.url, 'GET');
  console.log(`${item.url.split('/').pop()}  HEAD[${head}]  GET[${get}]`);
}
