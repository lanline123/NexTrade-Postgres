import { spawn } from 'node:child_process';
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
const nextBin = require.resolve('next/dist/bin/next');
const PORT = 3423;
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

const paths = ['/manifest.webmanifest', '/icon.png', '/robots.txt', '/sitemap.xml', '/apple-icon.png'];
for (const p of paths) {
  try {
    const res = await fetch(base + p, { signal: AbortSignal.timeout(20000), headers: { 'user-agent': 'probe' } });
    const ct = res.headers.get('content-type');
    const body = res.headers.get('content-type')?.includes('json') || p.endsWith('.webmanifest') ? (await res.text()).slice(0, 120) : '';
    console.log(`${p} -> ${res.status} ${ct} ${body}`);
  } catch (error) {
    console.log(`${p} -> error ${error.message}`);
  }
}
server.kill();
process.exit(0);
