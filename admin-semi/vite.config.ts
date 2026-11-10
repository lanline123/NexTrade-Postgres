import fs from 'node:fs';
import path from 'node:path';
import { defineConfig, type Plugin } from 'vite';
import react from '@vitejs/plugin-react';

const POS_ASSETS_DIR = process.env.POS_ASSETS_DIR || 'D:\\nextrade\\前端页面\\5.8.60\\o2o_admin\\admin\\assets';
const POS_PUBLIC_PATH = '/pos';
// POS 产物把路由 base 编译成了 `admin`，改空后以站点根为 base，才能稳定挂在 /pos。
const POS_BASE_LITERAL = 'VITE_BASE_DIR:`admin`';
const POS_BASE_PATCHED = 'VITE_BASE_DIR:``';
function patchPosBase(code: string): string {
  return code.split(POS_BASE_LITERAL).join(POS_BASE_PATCHED);
}

function posPreviewPlugin(): Plugin {
  return {
    name: 'pos-preview-static',
    configureServer(server) {
      if (!fs.existsSync(POS_ASSETS_DIR)) return;
      const files = fs.readdirSync(POS_ASSETS_DIR);
      const entry = files
        .filter((file) => /^index-.*\.js$/.test(file))
        .sort(
          (a, b) =>
            fs.statSync(path.join(POS_ASSETS_DIR, b)).size -
            fs.statSync(path.join(POS_ASSETS_DIR, a)).size,
        )[0];
      const cssFiles = files
        .filter((file) => /^index-.*\.css$/.test(file))
        .sort(
          (a, b) =>
            fs.statSync(path.join(POS_ASSETS_DIR, b)).size -
            fs.statSync(path.join(POS_ASSETS_DIR, a)).size,
        )
        .slice(0, 2);
      if (!entry) return;

      const html = `<!doctype html>
<html lang="zh-CN">
<head>
  <meta charset="UTF-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1.0" />
  <title>收银台</title>
  ${cssFiles.map((file) => `<link rel="stylesheet" href="${POS_PUBLIC_PATH}/assets/${file}" />`).join('\n  ')}
</head>
<body>
  <div id="app"></div>
  <script type="module" src="${POS_PUBLIC_PATH}/assets/${entry}"></script>
</body>
</html>`;

      server.middlewares.use((req, res, next) => {
        const rawUrl = req.url || '/';
        const pathname = decodeURIComponent(rawUrl.split('?')[0]);
        const referer = req.headers.referer || '';
        const isPosPath = pathname === POS_PUBLIC_PATH || pathname.startsWith(`${POS_PUBLIC_PATH}/`);
        const isPosLogin =
          (pathname === '/login/index' || pathname === '/admin/login/index') && rawUrl.includes('redirect=/pos');
        const isPosRedirect =
          isPosLogin ||
          (pathname.startsWith('/admin/assets/') && (referer.includes(POS_PUBLIC_PATH) || referer.includes('redirect=/pos')));
        if (!isPosPath && !isPosRedirect) return next();

        if (pathname.startsWith(`${POS_PUBLIC_PATH}/assets/`) || pathname.startsWith('/admin/assets/')) {
          const relative = pathname.startsWith(`${POS_PUBLIC_PATH}/assets/`)
            ? pathname.slice(`${POS_PUBLIC_PATH}/assets/`.length)
            : pathname.slice('/admin/assets/'.length);
          let filePath = path.join(POS_ASSETS_DIR, relative);
          if (!fs.existsSync(filePath) && relative.startsWith('assets/')) {
            filePath = path.join(POS_ASSETS_DIR, relative.slice('assets/'.length));
          }
          if (fs.existsSync(filePath) && fs.statSync(filePath).isFile()) {
            const ext = path.extname(filePath).toLowerCase();
            const types: Record<string, string> = {
              '.js': 'text/javascript',
              '.css': 'text/css',
              '.json': 'application/json',
              '.png': 'image/png',
              '.jpg': 'image/jpeg',
              '.jpeg': 'image/jpeg',
              '.webp': 'image/webp',
              '.svg': 'image/svg+xml',
              '.woff': 'font/woff',
              '.woff2': 'font/woff2',
              '.ttf': 'font/ttf',
            };
            res.setHeader('Content-Type', types[ext] || 'application/octet-stream');
            res.setHeader('Cache-Control', 'no-store');
            const raw = fs.readFileSync(filePath);
            res.end(ext === '.js' ? patchPosBase(raw.toString('utf8')) : raw);
            return;
          }
        }

        res.setHeader('Content-Type', 'text/html; charset=utf-8');
        res.end(html);
      });
    },
  };
}

export default defineConfig({
  plugins: [react(), posPreviewPlugin()],
  build: {
    rollupOptions: {
      output: {
        manualChunks: {
          'vendor-semi': ['@douyinfe/semi-ui', '@douyinfe/semi-icons', '@douyinfe/semi-foundation'],
          'vendor-react': ['react', 'react-dom'],
          'vendor-editor': ['@wangeditor/editor', '@wangeditor/editor-for-react'],
          'vendor-charts': ['echarts'],
        },
      },
    },
  },
  resolve: {
    alias: {
      '@semi-css': path.resolve(__dirname, 'node_modules/@douyinfe/semi-ui/dist/css/semi.min.css'),
    },
  },
  server: {
    port: 3002,
    proxy: {
      '/adminapi': {
        target: 'https://battery.recy.cc',
        changeOrigin: true,
        secure: false,
      },
      '/static': {
        target: 'https://battery.recy.cc',
        changeOrigin: true,
        secure: false,
      },
      // IM 客服域走根级契约路径（不带 /adminapi），生产 nginx 按
      // X-Client-Type: admin 头分流到管理端 8080
      '/im': {
        target: 'https://battery.recy.cc',
        changeOrigin: true,
        secure: false,
      },
      '/ws': {
        target: 'https://battery.recy.cc',
        changeOrigin: true,
        secure: false,
        ws: true,
      },
    },
  },
});
