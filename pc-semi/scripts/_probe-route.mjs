// 复现某个路由的 500：node scripts/_probe-route.mjs /shop
// 起一次 next start（临时端口），取页面状态 + 服务端报错，然后关掉。
import { spawn } from 'node:child_process';
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
const route = process.argv[2] || '/shop';
const port = Number(process.argv[3] || 3199);
const root = process.cwd();
const nextBin = require.resolve('next/dist/bin/next');

const server = spawn(process.execPath, [nextBin, 'start', '-p', String(port), '-H', '127.0.0.1'], {
  cwd: root,
  env: { ...process.env, NODE_ENV: 'production' },
});
let log = '';
server.stdout.on('data', (chunk) => { log += chunk.toString(); });
server.stderr.on('data', (chunk) => { log += chunk.toString(); });

const base = `http://127.0.0.1:${port}`;
const deadline = Date.now() + 90000;
let up = false;
while (Date.now() < deadline) {
  try {
    const response = await fetch(`${base}/`, { signal: AbortSignal.timeout(5000) });
    if (response.status < 500) { up = true; break; }
  } catch {}
  await new Promise((resolve) => setTimeout(resolve, 500));
}
if (!up) {
  server.kill();
  console.log('server did not start');
  console.log(log.slice(-4000));
  process.exit(0);
}

try {
  const response = await fetch(`${base}${route}`, { signal: AbortSignal.timeout(45000) });
  const body = await response.text();
  console.log(`status ${response.status} bytes ${body.length}`);
  const title = /<title>([^<]*)<\/title>/.exec(body);
  console.log(`title: ${title ? title[1] : '(none)'}`);
  const missing = /Missing: (\{[^}]*\})/.exec(body);
  if (missing) console.log(`missing: ${missing[1]}`);
  console.log('--- body head');
  console.log(body.slice(0, 1200));
} catch (error) {
  console.log(`fetch failed: ${error.message}`);
}

await new Promise((resolve) => setTimeout(resolve, 1500));
server.kill('SIGKILL');
console.log('--- server log tail');
console.log(log.slice(-6000));
