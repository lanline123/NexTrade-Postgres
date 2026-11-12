import { defineNitroPlugin } from "nitropack/runtime";
import { SITEMAP_CHANGEFREQ, SITEMAP_PRIORITY } from "../utils/sitemap";

const parseSitemapUrl = (loc: string) => {
    try {
        return new URL(loc, "https://placeholder.local");
    } catch {
        return null;
    }
};

const normalizePathname = (pathname: string) => {
    if (!pathname || pathname === "/") {
        return "/";
    }
    return pathname.replace(/\/+$/, "");
};

const resolveSitemapMeta = (loc: string) => {
    const parsedUrl = parseSitemapUrl(loc);
    if (!parsedUrl) {
        return null;
    }

    const pathname = normalizePathname(parsedUrl.pathname);
    const { searchParams } = parsedUrl;

    if (pathname === "/") {
        return {
            priority: SITEMAP_PRIORITY.home,
            changefreq: SITEMAP_CHANGEFREQ.home
        };
    }

    if (pathname.startsWith("/item/") || pathname.startsWith("/product/")) {
        return {
            priority: SITEMAP_PRIORITY.product,
            changefreq: SITEMAP_CHANGEFREQ.product
        };
    }

    if (pathname === "/list" && searchParams.has("cat")) {
        return {
            priority: SITEMAP_PRIORITY.category,
            changefreq: SITEMAP_CHANGEFREQ.category
        };
    }

    if (pathname.startsWith("/category/")) {
        return {
            priority: SITEMAP_PRIORITY.category,
            changefreq: SITEMAP_CHANGEFREQ.category
        };
    }

    if (pathname === "/shop/list") {
        return {
            priority: SITEMAP_PRIORITY.shop,
            changefreq: SITEMAP_CHANGEFREQ.shop
        };
    }

    if (pathname.startsWith("/shop/")) {
        return {
            priority: SITEMAP_PRIORITY.shop,
            changefreq: SITEMAP_CHANGEFREQ.shop
        };
    }

    if (pathname === "/seckill/list") {
        return {
            priority: SITEMAP_PRIORITY.landing,
            changefreq: SITEMAP_CHANGEFREQ.home
        };
    }

    if (pathname === "/article/issue/index") {
        return {
            priority: SITEMAP_PRIORITY.landing,
            changefreq: "weekly" as const
        };
    }

    if (pathname === "/article/issue/info" && searchParams.has("categoryId")) {
        return {
            priority: SITEMAP_PRIORITY.landing,
            changefreq: "weekly" as const
        };
    }

    if (pathname === "/article/news/list" && !searchParams.has("articleId")) {
        return {
            priority: SITEMAP_PRIORITY.landing,
            changefreq: "weekly" as const
        };
    }

    if (pathname.startsWith("/article/")) {
        return {
            priority: SITEMAP_PRIORITY.article,
            changefreq: SITEMAP_CHANGEFREQ.article
        };
    }

    return null;
};

export default defineNitroPlugin((nitroApp) => {
    nitroApp.hooks.hook("sitemap:resolved", (ctx) => {
        ctx.urls = ctx.urls.map((url) => {
            const meta = resolveSitemapMeta(url.loc);
            if (!meta) {
                return url;
            }

            return {
                ...url,
                priority: meta.priority,
                changefreq: meta.changefreq
            };
        });
    });
});
