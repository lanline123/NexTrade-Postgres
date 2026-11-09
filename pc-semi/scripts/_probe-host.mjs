// 探测图片主机可达性：node scripts/_probe-host.mjs <url> [url...]
const urls = process.argv.slice(2);
if (!urls.length) {
  console.log('usage: node scripts/_probe-host.mjs <url>...');
  process.exit(0);
}
for (const url of urls) {
  const started = Date.now();
  try {
    const controller = new AbortController();
    const timer = setTimeout(() => controller.abort(), 12000);
    const response = await fetch(url, { signal: controller.signal, method: 'GET' });
    clearTimeout(timer);
    const buffer = await response.arrayBuffer();
    console.log(`${url} -> ${response.status} ${response.headers.get('content-type') || ''} ${buffer.byteLength}B ${Date.now() - started}ms`);
  } catch (error) {
    console.log(`${url} -> ERROR ${error.name}: ${error.message} (${Date.now() - started}ms)`);
  }
}
