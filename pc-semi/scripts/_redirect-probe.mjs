// 诊断：打印某个路由的重定向链（不自动跟随），看是不是死循环。
import { spawn } from 'node:child_process';
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
const nextBin = require.resolve('next/dist/bin/next');
const PORT = 3421;
const base = `http://127.0.0.1:${PORT}`;
const server = spawn(process.execPath, [nextBin, 'start', '-p', String(PORT), '-H', '127.0.0.1'], {
  stdio: 'ignore',
  env: { ...process.env, NODE_ENV: 'production' },
});
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

for (let i = 0; i < 120; i += 1) {
  try {
    const res = await fetch(`${base}/`, { signal: AbortSignal.timeout(3000) });
    if (res.status < 500) break;
  } catch {
    /* not up */
  }
  await sleep(500);
}

let current = process.argv[2] || '/member/coupon/list';
const cookies = [];
for (let hop = 0; hop < 10; hop += 1) {
  const res = await fetch(base + current, {
    redirect: 'manual',
    signal: AbortSignal.timeout(30000),
    headers: cookies.length ? { cookie: cookies.join('; ') } : {},
  });
  const setCookie = res.headers.getSetCookie?.() || [];
  for (const value of setCookie) cookies.push(value.split(';')[0]);
  const location = res.headers.get('location');
  console.log(`${hop}: ${current} -> ${res.status}${location ? ` Location: ${location}` : ''}`);
  if (!location || res.status < 300 || res.status >= 400) break;
  current = location.startsWith('http') ? new URL(location).pathname + new URL(location).search : location;
}

server.kill();
process.exit(0);
