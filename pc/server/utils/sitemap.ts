import { createCipheriv } from "node:crypto";
import { getHeader, getRequestURL, type H3Event } from "h3";
import { buildCategoryPath, buildProductPath, resolveSeoPath } from "../../utils/seo-url";
import { buildShopPath } from "../../utils/seo-url";

const REQUEST_SECRET_KEY = "NexTradeApiSecret";
const DEFAULT_PAGE_SIZE = 200;
const DEFAULT_SHOP_PAGE_SIZE = 100;
const MAX_PAGE_COUNT = 100;
/** sitemap 内部 $fetch 超时（毫秒），避免后端慢响应挂住 Nitro 工人 */
const SITEMAP_FETCH_TIMEOUT_MS = 8000;

interface ApiResponse<T> {
    code: number;
    msg?: string;
    message?: string;
    data: T;
}

interface PagedResult<T> {
    records?: T[];
    total?: number;
}

export interface SitemapEntry {
    loc: string;
    priority: number;
    changefreq: "always" | "hourly" | "daily" | "weekly" | "monthly" | "yearly" | "never";
    lastmod?: string;
}

export const SITEMAP_PRIORITY = {
    home: 1.0,
    product: 0.9,
    category: 0.8,
    shop: 0.7,
    landing: 0.6,
    article: 0.5
} as const;

export const SITEMAP_CHANGEFREQ = {
    home: "daily",
    product: "daily",
    category: "weekly",
    shop: "weekly",
    landing: "weekly",
    article: "monthly"
} as const;

export const NEWS_CATEGORY_SNS = ["zxzx", "bzgg", "kfzx"] as const;

interface ProductRecord {
    productSn?: string;
    productStatus?: number;
    isDelete?: number;
    seoLink?: string | null;
    lastUpdate?: number | string;
    addTime?: number | string;
}

interface SeoCategoryRecord {
    id?: string | number;
    seoLink?: string | null;
    lastUpdate?: number | string;
}

interface SeoArticleRecord {
    id?: string | number;
    articleId?: string | number;
    seoLink?: string | null;
    loc?: string | null;
    path?: string | null;
    route?: string | null;
    lastUpdate?: number | string;
    addTime?: number | string;
}

interface ShopRecord {
    id?: string | number;
    shopId?: string | number;
    seoLink?: string | null;
    status?: number;
    lastUpdate?: number | string;
    addTime?: number | string;
}

interface CategoryFloorResult {
    catFloor?: CategoryFloorItem[];
}

interface HelpArticleRecord {
    articleId?: number;
    addTime?: number | string;
}

interface HelpCategoryRecord {
    categorySn?: string;
    articles?: HelpArticleRecord[];
}

interface NewsArticleRecord {
    articleId?: number;
    addTime?: string;
}

type CategoryNode = {
    categoryId?: number | string;
    children?: CategoryNode[];
};

type CategoryFloorItem = {
    catList?: Array<{
        id?: number | string;
        children?: CategoryNode[];
    }>;
};

const trimTrailingSlash = (value: string) => value.replace(/\/+$/, "");

const ensureLeadingSlash = (value: string) => (value.startsWith("/") ? value : `/${value}`);

const normalizeExternalUrl = (value: string, fallbackOrigin: string) => {
    if (!value) {
        return "";
    }
    if (/^https?:\/\//i.test(value)) {
        return trimTrailingSlash(value);
    }
    if (value.startsWith("//")) {
        return `https:${trimTrailingSlash(value)}`;
    }
    if (value.startsWith("/")) {
        return `${trimTrailingSlash(fallbackOrigin)}${value}`;
    }
    return `https://${trimTrailingSlash(value)}`;
};

export const buildApiBaseUrl = (apiUrl: string, apiPrefix: string, fallbackOrigin: string) => {
    const normalizedPrefix = ensureLeadingSlash(apiPrefix || "/api");
    const normalizedOrigin = apiUrl ? normalizeExternalUrl(apiUrl, fallbackOrigin) : trimTrailingSlash(fallbackOrigin);
    return `${normalizedOrigin}${normalizedPrefix.replace(/\/$/, "")}`;
};

export const resolveSiteOrigin = (siteUrl: string, fallbackOrigin: string, pcDomain?: string) => {
    if (siteUrl) {
        return normalizeExternalUrl(siteUrl, fallbackOrigin);
    }
    if (pcDomain) {
        return normalizeExternalUrl(pcDomain, fallbackOrigin);
    }
    return trimTrailingSlash(fallbackOrigin);
};

export const buildAbsoluteUrl = (siteOrigin: string, path: string) => {
    if (!path) {
        return siteOrigin;
    }
    return `${trimTrailingSlash(siteOrigin)}${ensureLeadingSlash(path)}`;
};

export const buildApiHeaders = (localeCode: string) => {
    const currentTimeStamp = Math.floor(Date.now() / 1000);
    const cipher = createCipheriv("aes-128-ecb", Buffer.from(REQUEST_SECRET_KEY, "utf8"), null);
    cipher.setAutoPadding(true);
    const secret = cipher.update(String(currentTimeStamp), "utf8", "base64") + cipher.final("base64");

    return {
        "Content-Type": "application/json",
        "X-Client-Type": "pc",
        "X-Locale-Code": localeCode || "zh-CN",
        Secret: secret
    };
};

export const fetchApiData = async <T>(
    apiBaseUrl: string,
    endpoint: string,
    headers: Record<string, string>,
    params?: Record<string, string | number | boolean | undefined | null>
) => {
    const query = new URLSearchParams();
    Object.entries(params || {}).forEach(([key, value]) => {
        if (value === undefined || value === null || value === "") {
            return;
        }
        query.set(key, String(value));
    });

    const requestUrl = query.size > 0 ? `${apiBaseUrl}/${endpoint}?${query.toString()}` : `${apiBaseUrl}/${endpoint}`;
    const result = await $fetch<ApiResponse<T>>(requestUrl, { headers, timeout: SITEMAP_FETCH_TIMEOUT_MS });

    if (result.code !== 0) {
        throw new Error(result.message || result.msg || `${endpoint} 请求失败`);
    }

    return result.data;
};

export const fetchPagedRecords = async <T>(
    apiBaseUrl: string,
    endpoint: string,
    headers: Record<string, string>,
    params?: Record<string, string | number | boolean | undefined | null>,
    pageSize: number = DEFAULT_PAGE_SIZE
) => {
    const records: T[] = [];
    let total = 0;

    for (let page = 1; page <= MAX_PAGE_COUNT; page++) {
        const result = await fetchApiData<PagedResult<T>>(apiBaseUrl, endpoint, headers, {
            ...params,
            page,
            size: pageSize
        });

        const currentRecords = Array.isArray(result.records) ? result.records : [];
        total = Number(result.total || 0);
        records.push(...currentRecords);

        if (currentRecords.length < pageSize || (total > 0 && records.length >= total)) {
            break;
        }
    }

    return records;
};

const walkCategoryNodes = (nodes: CategoryNode[] | undefined, idSet: Set<number>) => {
    if (!Array.isArray(nodes)) {
        return;
    }

    nodes.forEach((node) => {
        const categoryId = Number(node.categoryId);
        if (Number.isFinite(categoryId) && categoryId > 0) {
            idSet.add(categoryId);
        }
        walkCategoryNodes(node.children, idSet);
    });
};

export const collectCategoryIds = (catFloor: CategoryFloorItem[] = []) => {
    const idSet = new Set<number>();

    catFloor.forEach((item) => {
        item.catList?.forEach((cat) => {
            const topCategoryId = Number(cat.id);
            if (Number.isFinite(topCategoryId) && topCategoryId > 0) {
                idSet.add(topCategoryId);
            }
            walkCategoryNodes(cat.children, idSet);
        });
    });

    return [...idSet];
};

export const dedupeSitemapEntries = (entries: SitemapEntry[]) => {
    const entryMap = new Map<string, SitemapEntry>();
    entries.forEach((entry) => {
        if (!entry.loc) {
            return;
        }
        entryMap.set(entry.loc, entry);
    });
    return [...entryMap.values()];
};

export const trySitemapRequest = async <T>(handler: () => Promise<T>) => {
    try {
        return {
            success: true as const,
            data: await handler()
        };
    } catch {
        return {
            success: false as const
        };
    }
};

export const safeSitemapRequest = async <T>(handler: () => Promise<T>, fallbackValue: T) => {
    try {
        return await handler();
    } catch (error) {
        console.error("[sitemap]", error);
        return fallbackValue;
    }
};

export const fetchPreferredSitemapData = async <T>(preferredHandler: () => Promise<T>, fallbackHandler: () => Promise<T>, fallbackValue: T) => {
    const preferredResult = await trySitemapRequest(preferredHandler);
    if (preferredResult.success) {
        return preferredResult.data;
    }
    return safeSitemapRequest(fallbackHandler, fallbackValue);
};

const buildStaticEntries = (): SitemapEntry[] => [
    {
        loc: "/",
        priority: SITEMAP_PRIORITY.home,
        changefreq: SITEMAP_CHANGEFREQ.home
    },
    {
        loc: "/article/issue/index",
        priority: SITEMAP_PRIORITY.landing,
        changefreq: "weekly"
    },
    {
        loc: "/article/news/list",
        priority: SITEMAP_PRIORITY.landing,
        changefreq: "weekly"
    },
    {
        loc: "/shop/list",
        priority: SITEMAP_PRIORITY.shop,
        changefreq: SITEMAP_CHANGEFREQ.shop
    },
    {
        loc: "/seckill/list",
        priority: SITEMAP_PRIORITY.landing,
        changefreq: SITEMAP_CHANGEFREQ.home
    }
];

const buildNewsCategoryEntries = (): SitemapEntry[] =>
    NEWS_CATEGORY_SNS.map((categorySn) => ({
        loc: `/article/news/list?categoryId=${categorySn}`,
        priority: SITEMAP_PRIORITY.article,
        changefreq: "weekly"
    }));

const buildSeoArticlePath = (article: SeoArticleRecord) => {
    const articleId = article.articleId || article.id;
    const preferredPath = article.seoLink || article.loc || article.path || article.route;
    const fallbackPath = articleId ? `/article/news/list?articleId=${articleId}` : "";
    return resolveSeoPath(preferredPath, fallbackPath, {
        allowedPrefixes: ["/article"]
    });
};

export const collectSitemapEntries = async (event: H3Event) => {
    const runtimeConfig = useRuntimeConfig(event);
    const requestOrigin = getRequestURL(event).origin;
    const localeCode =
        getHeader(event, "x-locale-code") || getHeader(event, "accept-language")?.split(",")[0] || runtimeConfig.sitemapRuntime.localeCode || "zh-CN";

    const apiBaseUrl = buildApiBaseUrl(runtimeConfig.sitemapRuntime.apiUrl, runtimeConfig.sitemapRuntime.apiPrefix, requestOrigin);
    const apiHeaders = buildApiHeaders(localeCode);

    let shopsRequest: Promise<ShopRecord[]> = Promise.resolve([]);
    shopsRequest = fetchPreferredSitemapData(
        () => fetchApiData<ShopRecord[]>(apiBaseUrl, "sys/seo/shops", apiHeaders),
        () => fetchPagedRecords<ShopRecord>(apiBaseUrl, "shop/shop/list", apiHeaders, undefined, DEFAULT_SHOP_PAGE_SIZE),
        []
    );

    const [products, seoCategoriesResult, helpCategories, seoArticlesResult, shops] = await Promise.all([
        fetchPreferredSitemapData(
            () => fetchApiData<ProductRecord[]>(apiBaseUrl, "sys/seo/products", apiHeaders),
            () => fetchPagedRecords<ProductRecord>(apiBaseUrl, "product/product/list", apiHeaders, { productStatus: 1 }),
            []
        ),
        trySitemapRequest(() => fetchApiData<SeoCategoryRecord[]>(apiBaseUrl, "sys/seo/categories", apiHeaders)),
        safeSitemapRequest(
            () =>
                fetchApiData<HelpCategoryRecord[]>(apiBaseUrl, "article/category/indexBzzxList", apiHeaders, {
                    categorySize: 99,
                    articleSize: 99
                }),
            []
        ),
        trySitemapRequest(() => fetchApiData<SeoArticleRecord[]>(apiBaseUrl, "sys/seo/articles", apiHeaders)),
        shopsRequest
    ]);

    const productRecords = Array.isArray(products) ? products : [];
    const shopRecords = Array.isArray(shops) ? shops : [];
    const seoCategories = seoCategoriesResult.success && Array.isArray(seoCategoriesResult.data) ? seoCategoriesResult.data : null;
    const seoArticles = seoArticlesResult.success && Array.isArray(seoArticlesResult.data) ? seoArticlesResult.data : null;

    const [categoryFloor, newsArticleGroups] = await Promise.all([
        seoCategories
            ? Promise.resolve<CategoryFloorResult | null>(null)
            : safeSitemapRequest(() => fetchApiData<CategoryFloorResult>(apiBaseUrl, "common/pc/getCatFloor", apiHeaders), { catFloor: [] }),
        seoArticles
            ? Promise.resolve<NewsArticleRecord[][]>([])
            : Promise.all(
                  NEWS_CATEGORY_SNS.map((categorySn) =>
                      safeSitemapRequest(
                          () =>
                              fetchPagedRecords<NewsArticleRecord>(
                                  apiBaseUrl,
                                  "article/article/list",
                                  apiHeaders,
                                  {
                                      categorySn
                                  },
                                  100
                              ),
                          []
                      )
                  )
              )
    ]);

    const entries: SitemapEntry[] = [...buildStaticEntries(), ...buildNewsCategoryEntries()];

    productRecords.forEach((product) => {
        if (!product.productSn) {
            return;
        }

        if (product.productStatus !== undefined && Number(product.productStatus) !== 1) {
            return;
        }

        if (Number(product.isDelete) === 1) {
            return;
        }

        entries.push({
            loc: buildProductPath({
                seoLink: product.seoLink,
                sn: product.productSn
            }),
            priority: SITEMAP_PRIORITY.product,
            changefreq: SITEMAP_CHANGEFREQ.product,
            lastmod: toLastmod(product.lastUpdate || product.addTime)
        });
    });

    if (seoCategories) {
        seoCategories.forEach((category) => {
            if (!category.id) {
                return;
            }

            entries.push({
                loc: buildCategoryPath({
                    seoLink: category.seoLink,
                    id: category.id
                }),
                priority: SITEMAP_PRIORITY.category,
                changefreq: SITEMAP_CHANGEFREQ.category,
                lastmod: toLastmod(category.lastUpdate)
            });
        });
    } else {
        collectCategoryIds(categoryFloor?.catFloor || []).forEach((categoryId) => {
            entries.push({
                loc: buildCategoryPath({
                    id: categoryId
                }),
                priority: SITEMAP_PRIORITY.category,
                changefreq: SITEMAP_CHANGEFREQ.category
            });
        });
    }

    helpCategories.forEach((category) => {
        if (category.categorySn) {
            entries.push({
                loc: `/article/issue/info?categoryId=${category.categorySn}`,
                priority: SITEMAP_PRIORITY.article,
                changefreq: "weekly"
            });
        }

        category.articles?.forEach((article) => {
            if (!article.articleId) {
                return;
            }
            entries.push({
                loc: `/article/issue/info?articleId=${article.articleId}`,
                priority: SITEMAP_PRIORITY.article,
                changefreq: SITEMAP_CHANGEFREQ.article,
                lastmod: toLastmod(article.addTime)
            });
        });
    });

    if (seoArticles) {
        seoArticles.forEach((article) => {
            const loc = buildSeoArticlePath(article);
            if (!loc) {
                return;
            }

            entries.push({
                loc,
                priority: SITEMAP_PRIORITY.article,
                changefreq: SITEMAP_CHANGEFREQ.article,
                lastmod: toLastmod(article.lastUpdate || article.addTime)
            });
        });
    } else {
        newsArticleGroups.flat().forEach((article) => {
            if (!article.articleId) {
                return;
            }
            entries.push({
                loc: `/article/news/list?articleId=${article.articleId}`,
                priority: SITEMAP_PRIORITY.article,
                changefreq: SITEMAP_CHANGEFREQ.article,
                lastmod: toLastmod(article.addTime)
            });
        });
    }

    shopRecords.forEach((shop) => {
        const shopId = shop.shopId || shop.id;
        if (!shopId) {
            return;
        }

        if (shop.status !== undefined && Number(shop.status) !== 1) {
            return;
        }

        entries.push({
            loc: buildShopPath({
                seoLink: shop.seoLink,
                id: shopId
            }),
            priority: SITEMAP_PRIORITY.shop,
            changefreq: SITEMAP_CHANGEFREQ.shop,
            lastmod: toLastmod(shop.lastUpdate || shop.addTime)
        });
    });

    return dedupeSitemapEntries(entries);
};

export const toLastmod = (value?: string | number | null) => {
    if (value === undefined || value === null || value === "") {
        return undefined;
    }

    let date: Date;

    if (typeof value === "number") {
        const timestamp = value < 1_000_000_000_000 ? value * 1000 : value;
        date = new Date(timestamp);
    } else if (/^\d+$/.test(value)) {
        const numericValue = Number(value);
        const timestamp = numericValue < 1_000_000_000_000 ? numericValue * 1000 : numericValue;
        date = new Date(timestamp);
    } else {
        date = new Date(value);
    }

    if (Number.isNaN(date.getTime())) {
        return undefined;
    }

    return date.toISOString();
};

const xmlEscape = (value: string) =>
    value
        .replace(/&/g, "&amp;")
        .replace(/"/g, "&quot;")
        .replace(/'/g, "&apos;")
        .replace(/</g, "&lt;")
        .replace(/>/g, "&gt;");

export const buildSitemapXml = (entries: SitemapEntry[]) => {
    const body = entries
        .map((entry) => {
            const parts = [
                "  <url>",
                `    <loc>${xmlEscape(entry.loc)}</loc>`,
                `    <changefreq>${entry.changefreq}</changefreq>`,
                `    <priority>${entry.priority.toFixed(1)}</priority>`
            ];

            if (entry.lastmod) {
                parts.splice(2, 0, `    <lastmod>${entry.lastmod}</lastmod>`);
            }

            parts.push("  </url>");
            return parts.join("\n");
        })
        .join("\n");

    return `<?xml version="1.0" encoding="UTF-8"?>\n<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">\n${body}\n</urlset>`;
};
