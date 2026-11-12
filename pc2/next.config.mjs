/** @type {import('next').NextConfig} */

// 后端口径与 pc-semi 保持一致；可用 PC2_API_BASE 覆盖。
const apiBase = (process.env.PC2_API_BASE || 'https://battery.recy.cc').replace(/\/$/, '');

const nextConfig = {
    // 与 Nuxt 的 .nuxt / .output 以及仓库内其它 .next 隔离，支持并行会话各自构建。
    distDir: process.env.PC2_NEXT_DIST_DIR || '.next-react',
    reactStrictMode: true,
    poweredByHeader: false,

    // 根目录 tsconfig.json 属于 Nuxt（extends .nuxt/tsconfig.json），不能被 Next 改写。
    // Next 14 支持自定义 tsconfig 路径，React 侧单独一份。
    typescript: { tsconfigPath: 'tsconfig.react.json' },

    // 根 pages/ 是 Vue 的 Nuxt 页面目录，里面的 .ts 会带 Nuxt 的 ~/ alias。
    // 只让 React 扩展名参与页面发现，Vue 侧一个文件都不用移动，也不被 Next 编译。
    // 坑：只写 ['tsx'] 单元素会触发 Next 14 内部 `pageExtensions.map is not a function`，
    // 至少要给两个元素（jsx 在这里没有实际文件，仅用于凑成数组）。
    pageExtensions: ['tsx', 'jsx'],

    env: {
        PC2_API_BASE: apiBase,
        PC2_API_PREFIX: process.env.PC2_API_PREFIX || '/api'
    },

    images: {
        remotePatterns: [
            { protocol: 'https', hostname: '**' },
            { protocol: 'http', hostname: '**' }
        ]
    },

    async rewrites() {
        return [
            { source: '/api/:path*', destination: `${apiBase}/api/:path*` },
            { source: '/adminapi/:path*', destination: `${apiBase}/adminapi/:path*` },
            { source: '/static/:path*', destination: `${apiBase}/static/:path*` }
        ];
    }
};

export default nextConfig;
