// https://nuxt.com/docs/api/configuration/nuxt-config
import { createSvgIconsPlugin } from "vite-plugin-svg-icons";
import viteCompression from "vite-plugin-compression";
import svgLoader from "vite-svg-loader";
import path from "path";
import { loadEnv } from "vite";
import notFoundStatus from "./modules/not-found-status";

const sitemapSiteUrl = process.env.VITE_API_URL;
// robots 的 demo 判断必须与客户端 import.meta.env.VITE_IS_DEMO 同源：客户端走 Vite loadEnv（含 .env），
// 而 nuxt --dotenv 只把 .env.production 注入 process.env；CI 把 VITE_IS_DEMO 写在 .env 时后者读不到，故用 loadEnv 对齐。
const isDemoBuild = (process.env.VITE_IS_DEMO ?? loadEnv(process.env.NODE_ENV || "production", process.cwd(), "VITE_").VITE_IS_DEMO) === "1";

export default defineNuxtConfig({
    // 开发缓存与 build/generate/prepare 产物分离，避免运行中的 dev 目录被清理。
    buildDir: process.env.NODE_ENV === "development" ? ".nuxt-dev" : ".nuxt",

    devtools: { enabled: true },

    ssr: process.env.VITE_SSR === "1",

    site: {
        url: sitemapSiteUrl,
        name: "NexTrade O2O PC"
    },

    runtimeConfig: {
        isDemo: isDemoBuild,
        sitemapRuntime: {
            siteUrl: sitemapSiteUrl,
            apiUrl: process.env.VITE_API_URL,
            apiPrefix: process.env.VITE_API_PREFIX || "/api",
            localeCode: "zh-CN"
        }
    },

    // 构建性能优化
    experimental: {
        payloadExtraction: false // 禁用 payload 提取可以加速构建
    },

    devServer: {
        host: "0.0.0.0",
        port: 3000
    },

    app: {
        baseURL: process.env.VITE_APP_BASE_URL || "/" // 配置基础 URL，空值回退为根路径避免产生 //
    },

    nitro: {
        // Windows 构建时内联 Nuxt 渲染器，确保 manifest 虚拟模块能够加载。
        externals: {
            inline: process.platform === "win32" ? [/[\\/]node_modules[\\/]nuxt[\\/]dist[\\/]/] : []
        },
        // generate 静态构建下已无 public/robots.txt，需显式预渲染此 server route，否则 /robots.txt 会 404。
        // SSR 构建有运行时 server，robots.txt.ts 会按请求响应，无需预渲染；且预渲染器会在 Nitro 打包前
        // 把整个应用装进同一个 Node 进程，融合后的代码量下这部分残留内存会把默认 4G 堆撑爆。
        prerender: {
            routes: process.env.VITE_SSR === "1" ? [] : ["/robots.txt"]
        },
        // 该配置用于服务端请求转发
        routeRules: {
            "/api/**": {
                //  如果需要做代理转发，使用proxy
                proxy: process.env.VITE_PROXY === "1" ? process.env.VITE_API_URL + "/api/**" : undefined,
                ssr: false
            }
        },
        compressPublicAssets: true, // 压缩静态资源
        minify: true, // 压缩服务端代码
        sourceMap: false // 禁用 source map
    },

    modules: [notFoundStatus, "@element-plus/nuxt", "@pinia/nuxt", "@nuxtjs/i18n", "@nuxtjs/sitemap"],

    sitemap: {
        sources: ["/api/__sitemap__/urls"],
        cacheMaxAgeSeconds: 3600,
        excludeAppSources: true,
        autoI18n: false,
        debug: process.env.NODE_ENV !== "production",
        exclude: [
            "/member/**",
            "/order/**",
            "/cart/**",
            "/chat/**",
            "/search/**",
            "/shop/search/**",
            "/store/search/**",
            "/store/**",
            "/install/**",
            "/preview",
            "/verify-code",
            "/pcMobile/**",
            "/checkout/**",
            "/login/**",
            "/register/**"
        ]
    },

    imports: {
        presets: [
            {
                from: "ant-design-vue",
                imports: ["message"]
            }
        ]
    },

    css: [
        "~/assets/css/global.less", // 相对路径，从项目根目录开始
        "~/assets/css/member/member-width.less" // 会员中心宽度对齐跨境版(1264)覆盖，仅 /member 路由生效，不改 global.less
    ],

    i18n: {
        strategy: "no_prefix", // 添加路由前缀的方式
        vueI18n: "~/i18n/config.ts"
    },

    vite: {
        css: {
            // 工作线程解析 @import 时主线程未及时回应会报 timed-out，改在主线程编译
            preprocessorMaxWorkers: 0
        },
        build: {
            minify: "esbuild",
            sourcemap: false
        },
        plugins: [
            createSvgIconsPlugin({
                iconDirs: [path.resolve(process.cwd(), "assets/icons")],
                symbolId: "icon-[name]"
            }),
            // 让 SVG 支持通过 ?component 后缀作为 Vue 组件导入，供 VortIcon 使用；
            // defaultImport: "url" 保证既有 <SvgIcon> 走 vite-plugin-svg-icons 的 symbol 引用不受影响
            svgLoader({
                defaultImport: "url",
                svgoConfig: {
                    multipass: true,
                    plugins: [
                        {
                            name: "preset-default",
                            params: {
                                overrides: {
                                    removeViewBox: false,
                                    removeUselessStrokeAndFill: false
                                }
                            }
                        }
                    ]
                }
            }),
            ...(process.env.NODE_ENV === "production"
                ? [
                      viteCompression({
                          verbose: false, // 不输出压缩日志
                          algorithm: "gzip",
                          ext: ".gz",
                          threshold: 10240 // 10KB 以上才压缩
                      })
                  ]
                : [])
        ]
    },

    compatibilityDate: "2024-10-25"
});
