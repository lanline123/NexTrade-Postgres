import { defineEventHandler, getRequestURL, setHeader } from "h3";

const DISALLOW_RULES = [
    "User-agent: *",
    "Allow: /",
    "Disallow: /member/",
    "Disallow: /order/",
    "Disallow: /cart/",
    "Disallow: /chat/",
    "Disallow: /install/",
    "Disallow: /pcMobile/",
    "Disallow: /preview",
    "Disallow: /verify-code",
    "Disallow: /checkout/",
    "Disallow: /login/",
    "Disallow: /register/"
];

// 演示站禁止全站收录，避免 demo 数据被搜索引擎索引
const BLOCKED_ROBOTS = ["User-agent: *", "Disallow: /"].join("\n");

const LOCAL_ORIGIN_RE = /^https?:\/\/(localhost|127\.0\.0\.1|0\.0\.0\.0|\[::1\])(:|$)/i;

/**
 * Sitemap 声明地址
 * @description 规范要求绝对地址，各语言域名各自指向自己的 sitemap。
 *  预渲染与本地调试取不到真实域名，此时回落相对地址，避免把 localhost 写进产物。
 */
const resolveSitemapUrl = (origin: string) => {
    if (!origin || LOCAL_ORIGIN_RE.test(origin)) return "/sitemap.xml";
    return `${origin}/sitemap.xml`;
};

export default defineEventHandler((event) => {
    setHeader(event, "Content-Type", "text/plain; charset=utf-8");
    const { isDemo } = useRuntimeConfig(event);

    if (isDemo) return `${BLOCKED_ROBOTS}\n`;

    // 与后端判定访问域名的口径保持一致：优先 X-Forwarded-Host，其次 Host
    const { origin } = getRequestURL(event, { xForwardedHost: true, xForwardedProto: true });
    return `${[...DISALLOW_RULES, `Sitemap: ${resolveSitemapUrl(origin)}`].join("\n")}\n`;
});
