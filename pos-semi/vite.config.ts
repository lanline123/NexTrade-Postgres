import react from '@vitejs/plugin-react';
import { defineConfig, loadEnv } from 'vite';

export default defineConfig(({ mode }) => {
  const env = loadEnv(mode, process.cwd(), '');
  // 独立部署：默认挂在站点 /pos；换路径或独立子域时在 .env.local 里设 VITE_BASE_PATH（如 `/`）。
  const base = env.VITE_BASE_PATH || '/pos/';

  return {
    base,
    plugins: [react()],
    server: {
      port: 3003,
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
      },
    },
  };
});
