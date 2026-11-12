/** @type {import('next').NextConfig} */

// 后端口径与 pc-semi 保持一致；可用 PC_API_BASE 覆盖。
const apiBase = (process.env.PC_API_BASE || 'https://battery.recy.cc').replace(/\/$/, '');

const nextConfig = {
    // 与 Nuxt 的 .nuxt / .output 以及仓库内其它 .next 隔离，支持并行会话各自构建。
    distDir: process.env.PC_NEXT_DIST_DIR || '.next-react',
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
        PC_API_BASE: apiBase,
        PC_API_PREFIX: process.env.PC_API_PREFIX || '/api',
        // IM WebSocket 地址（客户端要用，必须是 NEXT_PUBLIC_ 前缀才注入）
        NEXT_PUBLIC_IM_WS: process.env.PC_IM_WS || ''
    },

    images: {
        remotePatterns: [
            { protocol: 'https', hostname: '**' },
            { protocol: 'http', hostname: '**' }
        ]
    },

    /**
     * 旧 URL → 新路由。Vue 侧这些路径要么是废弃版本（*.v1）、要么是 PC 端不再使用的支付渠道页。
     *
     * 注意：WebSocket（/ws）**不能**用 rewrites 代理（Next 不支持 upgrade 转发），
     * 线上需要 nginx 把 /ws 转到 IM 服务；或用环境变量 PC_IM_WS 指定完整 ws 地址。
     * 未列入的旧路径由根 404 / member 的 [...slug] 兜底承接，不会白屏。
     */
    async redirects() {
        return [
            { source: '/item/:id', destination: '/product/:id', permanent: true },
            { source: '/coupon', destination: '/coupon/list', permanent: false },
            { source: '/search.v1', destination: '/search', permanent: true },
            { source: '/cart/index.v1', destination: '/cart', permanent: false },
            { source: '/chat/index.v1', destination: '/', permanent: false },
            { source: '/pcmobile', destination: '/', permanent: false },
            { source: '/giftcard/transfer/:id', destination: '/member/giftCard/exchange', permanent: false },
            { source: '/join/intro', destination: '/join', permanent: false },
            { source: '/join/plan', destination: '/join', permanent: false },
            { source: '/join/process', destination: '/join', permanent: false },
            { source: '/join/standard', destination: '/join', permanent: false },
            { source: '/member/reset', destination: '/member/login', permanent: false },
            { source: '/member/security/dialoginfo', destination: '/member/security/info', permanent: false },
            { source: '/member/bindmobilephone', destination: '/member/security/info', permanent: false },
            { source: '/member/bindMobilePhone', destination: '/member/security/info', permanent: false },
            { source: '/exchange/:id', destination: '/product/:id', permanent: false },
            { source: '/join/joinheader', destination: '/join', permanent: false },
            { source: '/join/joinHeader', destination: '/join', permanent: false },
            {
                source: '/order/:channel(alipay|wechat|paypal|stripe|huifu|lakala|yabanpay|yeepay|yunpay)',
                destination: '/order/pay',
                permanent: false
            },
            {
                source:
                    '/activity/:module(pcImageAd|pcImageSquareAd|pcImageHotarea|pcImageNav|pcProduct|pcTitleAd|pcNotice|pcWhiteLine|pcWhiteBlank|pcCoupon|pcSeckill|pcGroupon|pcProductGrouping)',
                destination: '/activity',
                permanent: false
            }
        ];
    },

    async rewrites() {
        return [
            { source: '/api/:path*', destination: `${apiBase}/api/:path*` },
            // IM 接口不在 /api 下（Vue 用 { prefix: '/im/' } 换 baseURL），单独转发。
            { source: '/im/:path*', destination: `${apiBase}/im/:path*` },
            { source: '/adminapi/:path*', destination: `${apiBase}/adminapi/:path*` },
            { source: '/static/:path*', destination: `${apiBase}/static/:path*` }
        ];
    }
};

export default nextConfig;
