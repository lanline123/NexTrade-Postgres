const normalizeSeoPath = (value?: string | null) => {
    if (!value) {
        return "";
    }

    const trimmedValue = value.trim();
    if (!trimmedValue) {
        return "";
    }

    try {
        if (/^https?:\/\//i.test(trimmedValue)) {
            const parsedUrl = new URL(trimmedValue);
            return `${parsedUrl.pathname}${parsedUrl.search}${parsedUrl.hash}` || "/";
        }

        if (trimmedValue.startsWith("//")) {
            const parsedUrl = new URL(`https:${trimmedValue}`);
            return `${parsedUrl.pathname}${parsedUrl.search}${parsedUrl.hash}` || "/";
        }
    } catch {
        return "";
    }

    return trimmedValue.startsWith("/") ? trimmedValue : `/${trimmedValue}`;
};

const appendQuery = (path: string, query: Record<string, string | number | undefined | null>) => {
    if (!path) {
        return "";
    }

    const [pathname, search = ""] = path.split("?");
    const params = new URLSearchParams(search);

    Object.entries(query).forEach(([key, value]) => {
        if (value === undefined || value === null || value === "") {
            return;
        }
        params.set(key, String(value));
    });

    const queryString = params.toString();
    const normalizedPath = pathname || "/";
    return queryString ? `${normalizedPath}?${queryString}` : normalizedPath;
};

const isSupportedPathPrefix = (path: string, allowedPrefixes: string[]) => {
    return allowedPrefixes.some((prefix) => path === prefix || path.startsWith(`${prefix}/`) || path.startsWith(`${prefix}?`));
};

export const resolveSeoPath = (
    seoLink: string | null | undefined,
    fallbackPath: string,
    options?: {
        allowedPrefixes?: string[];
    }
) => {
    const normalizedSeoPath = normalizeSeoPath(seoLink);
    if (!normalizedSeoPath) {
        return fallbackPath;
    }

    if (options?.allowedPrefixes?.length && !isSupportedPathPrefix(normalizedSeoPath, options.allowedPrefixes)) {
        return fallbackPath;
    }

    return normalizedSeoPath;
};

export const extractProductSnFromSlug = (slug: string | string[] | null | undefined) => {
    const segments = (Array.isArray(slug) ? slug : typeof slug === "string" ? [slug] : []).map((item) => String(item).trim()).filter(Boolean);
    const lastSegment = segments[segments.length - 1] || "";
    if (!lastSegment) {
        return "";
    }

    const separatorIndex = lastSegment.lastIndexOf(".");
    if (separatorIndex < 0 || separatorIndex === lastSegment.length - 1) {
        return lastSegment;
    }

    return lastSegment.slice(separatorIndex + 1);
};

export const buildProductPath = ({
    seoLink,
    sn,
    skuId,
    type,
    grouponPromotionRecordId,
    shopId
}: {
    seoLink?: string | null;
    sn?: string | number | null;
    skuId?: string | number | null;
    type?: string | null;
    grouponPromotionRecordId?: string | number | null;
    shopId?: string | number | null;
}) => {
    const fallbackPath = sn ? `/item/${sn}` : "";
    const basePath = resolveSeoPath(seoLink, fallbackPath, {
        allowedPrefixes: ["/item", "/product"]
    });

    return appendQuery(basePath, {
        skuId,
        type,
        grouponPromotionRecordId,
        shopId
    });
};

export const buildCategoryPath = ({
    seoLink,
    id,
    pId
}: {
    seoLink?: string | null;
    id?: string | number | null;
    pId?: string | number | null;
}) => {
    const fallbackPath = id ? `/list/?cat=${id}` : "";
    const basePath = resolveSeoPath(seoLink, fallbackPath, {
        allowedPrefixes: ["/list", "/category"]
    });

    return appendQuery(basePath, {
        pId
    });
};

export const buildShopPath = ({
    seoLink,
    id
}: {
    seoLink?: string | null;
    id?: string | number | null;
}) => {
    const fallbackPath = id ? `/shop/${id}` : "";
    return resolveSeoPath(seoLink, fallbackPath, {
        allowedPrefixes: ["/shop"]
    });
};
