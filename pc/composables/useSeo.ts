import { useCommonStore } from "@/store/common";
import { useI18nStore } from "@/store/i18n";
import type { Ref, ComputedRef } from "vue";

type MaybeRef<T> = T | Ref<T> | ComputedRef<T>;

interface SeoOptions {
    title: MaybeRef<string>;
    description?: MaybeRef<string>;
    keywords?: MaybeRef<string>;
    image?: MaybeRef<string>;
    ogType?: MaybeRef<string>;
    canonical?: MaybeRef<string>;
    robots?: MaybeRef<string>;
    /** 是否追加站点标题后缀（shopTitleSuffix），默认 true */
    appendSuffix?: boolean;
    /** 是否追加 powered by 后缀，默认 true（受 poweredByStatus 控制） */
    appendPoweredBy?: boolean;
}

export const useSeo = (options: SeoOptions) => {
    const commonStore = useCommonStore();
    const { t } = useI18n();
    const requestUrl = useRequestURL();
    const absoluteOrigin = computed(() => requestUrl.origin);

    const fullTitle = computed(() => {
        const title = unref(options.title);
        if (!title) return commonStore.shopTitle || "";

        let result = title;

        if (options.appendSuffix !== false && commonStore.shopTitleSuffix) {
            result += " - " + t(commonStore.shopTitleSuffix);
        }

        if (options.appendPoweredBy !== false && commonStore.poweredByStatus != 1) {
            result += "-powered by nextrade";
        }

        return result;
    });

    const desc = computed(() => unref(options.description) || commonStore.shopDesc || "");
    const kw = computed(() => unref(options.keywords) || commonStore.shopKeywords || "");
    const canonical = computed(() => {
        const value = unref(options.canonical);
        if (!value) return "";
        if (/^https?:\/\//i.test(value)) return value;
        return new URL(value, requestUrl.origin).toString();
    });
    const robots = computed(() => unref(options.robots) || "");
    const siteName = computed(() => commonStore.shopTitle || commonStore.shopName || "");
    const imageUrl = computed(() => {
        const value = unref(options.image) || commonStore.shopLogo;
        if (!value) return "";
        if (/^https?:\/\//i.test(value)) return value;
        if (value.startsWith("//")) return `https:${value}`;
        if (value.startsWith("/")) return new URL(value, absoluteOrigin.value).toString();
        if (commonStore.storageUrl) {
            if (/^https?:\/\//i.test(commonStore.storageUrl)) {
                return `${commonStore.storageUrl.replace(/\/$/, "")}/${value.replace(/^\//, "")}`;
            }
            if (commonStore.storageUrl.startsWith("//")) {
                return `https:${commonStore.storageUrl.replace(/\/$/, "")}/${value.replace(/^\//, "")}`;
            }
        }
        return value;
    });
    const ogUrl = computed(() => canonical.value || requestUrl.href);

    useHead({
        title: fullTitle,
        meta: computed(() => [
            { name: "description", content: desc.value },
            { name: "keywords", content: kw.value },
            ...(robots.value ? [{ name: "robots", content: robots.value }] : []),
            { property: "og:title", content: fullTitle.value },
            { property: "og:description", content: desc.value },
            { property: "og:type", content: unref(options.ogType) || "website" },
            ...(siteName.value ? [{ property: "og:site_name", content: siteName.value }] : []),
            ...(ogUrl.value ? [{ property: "og:url", content: ogUrl.value }] : []),
            ...(imageUrl.value ? [{ property: "og:image", content: imageUrl.value }] : []),
            { name: "twitter:card", content: imageUrl.value ? "summary_large_image" : "summary" },
            { name: "twitter:title", content: fullTitle.value },
            { name: "twitter:description", content: desc.value },
            ...(imageUrl.value ? [{ name: "twitter:image", content: imageUrl.value }] : [])
        ]),
        link: computed(() =>
            canonical.value
                ? [
                      {
                          rel: "canonical",
                          href: canonical.value
                      }
                  ]
                : []
        )
    });

    // 各语言站点互相声明 alternate；未配置域名的语言不参与，避免指向不存在的地址
    const i18nStore = useI18nStore();
    useHead({
        link: computed(() => {
            const path = `${requestUrl.pathname}${requestUrl.search}`;
            const items = i18nStore.langList
                .map((item) => ({ hreflang: item.localeCode, domain: item.domain?.trim() || "", isDefault: item.isDefault }))
                .filter((item) => !!item.domain);

            if (!items.length) return [];

            const fallback = items.find((item) => item.isDefault === 1) ?? items[0];

            return [
                ...items.map((item) => ({ rel: "alternate", hreflang: item.hreflang, href: `https://${item.domain}${path}` })),
                { rel: "alternate", hreflang: "x-default", href: `https://${fallback.domain}${path}` }
            ];
        })
    });
};
