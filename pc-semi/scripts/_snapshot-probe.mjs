// 诊断：'next start' 起来后，某些路由是否会让服务进程直接挂掉（而不是慢）。
import { spawn } from 'node:child_process';
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
const nextBin = require.resolve('next/dist/bin/next');
const PORT = 3419;
const base = `http://127.0.0.1:${PORT}`;
const server = spawn(process.execPath, [nextBin, 'start', '-p', String(PORT), '-H', '127.0.0.1'], {
  stdio: ['ignore', 'ignore', 'pipe'],
  env: { ...process.env, NODE_ENV: 'production' },
});
let stderr = '';
server.stderr.on('data', (d) => {
  stderr += d.toString();
});
let exited = null;
server.on('exit', (code, signal) => {
  exited = { code, signal };
});

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

async function hit(path, timeoutMs = 60000) {
  const started = Date.now();
  try {
    const res = await fetch(base + path, { signal: AbortSignal.timeout(timeoutMs) });
    const body = await res.text();
    return { path, status: res.status, ms: Date.now() - started, bytes: body.length };
  } catch (error) {
    return { path, error: error.message, cause: error.cause?.message, ms: Date.now() - started };
  }
}

for (let i = 0; i < 120; i += 1) {
  try {
    const res = await fetch(`${base}/`, { signal: AbortSignal.timeout(3000) });
    if (res.status < 500) break;
  } catch {
    /* not up yet */
  }
  await sleep(500);
}

const targets = process.argv.slice(2);
for (const path of targets.length ? targets : ['/article/news/info', '/member/coupon/list', '/']) {
  const result = await hit(path);
  console.log(JSON.stringify(result), 'serverExited=', JSON.stringify(exited));
  await sleep(500);
}
console.log('--- server stderr tail ---');
console.log(stderr.slice(-2500));
server.kill();
process.exit(0);
