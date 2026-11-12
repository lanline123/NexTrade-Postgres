import { collectSitemapEntries } from "../../utils/sitemap";

/**
 * sitemap 数据源端点
 * - Nitro 路由级缓存 30 分钟，配合 swr 让冷缓存命中时先返回旧数据再异步刷新
 * - 与 nuxt.config.ts 中 @nuxtjs/sitemap 模块的 cacheMaxAgeSeconds 叠加，进一步抑制并发抓取压力
 * - getKey 包含 locale，避免多语言场景互相污染缓存
 */
export default defineCachedEventHandler(
    async (event) => {
        return collectSitemapEntries(event);
    },
    {
        maxAge: 1800,
        swr: true,
        getKey: (event) => {
            const localeCode =
                getHeader(event, "x-locale-code") ||
                getHeader(event, "accept-language")?.split(",")[0] ||
                "zh-CN";
            return `sitemap-urls:${localeCode}`;
        }
    }
);
