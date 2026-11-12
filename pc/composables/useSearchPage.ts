import { reactive, ref, computed, watchEffect, toValue, inject, type InjectionKey, type MaybeRefOrGetter, type Ref, type ComputedRef } from "vue";
import { useRouter } from "vue-router";
import { useI18n } from "vue-i18n";
import { useCommonStore } from "@/store/common";
import { buildSeoKeywords } from "@/utils/seo";
import type { filterSeleted, SearchFilter, QueryParams } from "@/types/search/search.d";

/**
 * @description 搜索页共享上下文：新旧两套 UI 共用同一份业务逻辑（唯一真理源）
 * - filter 状态、route<->filter 同步、queryParams 计算、SEO、筛选变更/撤销
 * - 布局组件只负责展示，通过 inject 消费本上下文，不再各自持有业务逻辑
 */
export interface SearchPageContext {
    pageType: ComputedRef<string>;
    filter: SearchFilter;
    filterSeleted: Ref<filterSeleted | undefined>;
    attrList: Ref<any[]>;
    queryParams: ComputedRef<QueryParams>;
    onFilterChange: (page?: number) => void;
    onFilterAttrChange: (attrs: any[]) => void;
    onFilterUndo: () => void;
    clearAttr: (name: string) => void;
}

export const SearchPageInjectionKey: InjectionKey<SearchPageContext> = Symbol("searchPageContext");

const parseAttrsQuery = (attrs: unknown): any[] => {
    if (!attrs) {
        return [];
    }
    try {
        const parsedAttrs = JSON.parse(String(attrs));
        return Array.isArray(parsedAttrs) ? parsedAttrs : [];
    } catch {
        return [];
    }
};

const serializeAttrsQuery = (attrs: any[]): string | undefined => {
    if (!Array.isArray(attrs) || attrs.length === 0) {
        return undefined;
    }
    return JSON.stringify(attrs);
};

/** 数字数组按内容比较，避免内容相同却因新引用触发响应式 */
const isSameNumberArray = (a: number[], b: number[]): boolean => {
    if (a === b) {
        return true;
    }
    if (!Array.isArray(a) || !Array.isArray(b) || a.length !== b.length) {
        return false;
    }
    return a.every((item, index) => item === b[index]);
};

/** 属性数组按内容比较（结构简单，直接序列化比对即可） */
const isSameAttrs = (a: any[], b: any[]): boolean => {
    if (a === b) {
        return true;
    }
    if (!Array.isArray(a) || !Array.isArray(b) || a.length !== b.length) {
        return false;
    }
    return JSON.stringify(a) === JSON.stringify(b);
};

/** 撤销「最后一步」属性选择：多值属性先去掉最后一个取值，否则去掉整行 */
const undoLastSelectedAttr = (attrs: any[]): any[] => {
    if (!Array.isArray(attrs) || attrs.length === 0) {
        return [];
    }
    const next = attrs.map((a) => ({
        attrName: a.attrName,
        attrValue: Array.isArray(a.attrValue) ? [...a.attrValue] : []
    }));
    const last = next[next.length - 1];
    if (last.attrValue.length > 1) {
        last.attrValue.pop();
    } else {
        next.pop();
    }
    return next.filter((a) => a.attrValue.length > 0);
};

export function useSearchPage(pageTypeInput: MaybeRefOrGetter<string>): SearchPageContext {
    const route = useRoute();
    const router = useRouter();
    const commonStore = useCommonStore();
    const { t } = useI18n();

    const pageType = computed(() => toValue(pageTypeInput) || "search");

    const attrList = ref<any[]>([]);
    const filter = reactive<SearchFilter>({
        page: 1,
        categoryId: 0,
        brandIds: [],
        sortField: "",
        sortOrder: "",
        maxPrice: null,
        minPrice: null,
        couponId: 0,
        keyword: ""
    });

    // 当路由参数变化时，同步更新 filter 的值
    // 注意：组件交互会先直接修改 filter（第一波响应式），随后 navigateTo 改变路由再触发本 watchEffect 回写 filter。
    // 若每次都给数组赋新引用，即使内容相同也会产生第二波伪响应式，导致 queryParams 重算、getFilter 等监听重复触发。
    // 因此对数组类字段按内容比较，内容未变则不重新赋值，避免重复请求。
    watchEffect(() => {
        filter.page = route.query.page ? Number(route.query.page) : 1;
        filter.categoryId = route.query.cat ? Number(route.query.cat) : 0;
        const nextBrandIds = route.query.brand ? String(route.query.brand).split(",").map(Number) : [];
        if (!isSameNumberArray(filter.brandIds, nextBrandIds)) {
            filter.brandIds = nextBrandIds;
        }
        filter.sortField = route.query.sort ? String(route.query.sort) : "";
        filter.sortOrder = route.query.order ? String(route.query.order) : "";
        filter.maxPrice = route.query.max ? Number(route.query.max) : null;
        filter.minPrice = route.query.min ? Number(route.query.min) : null;
        filter.keyword = route.query.keyword ? String(route.query.keyword) : "";
        filter.couponId = route.query.couponId ? Number(route.query.couponId) : 0;
        const nextAttrs = parseAttrsQuery(route.query.attrs);
        if (!isSameAttrs(attrList.value, nextAttrs)) {
            attrList.value = nextAttrs;
        }
    });

    const filterSeleted = ref<filterSeleted>();

    const queryParams = computed(() => {
        let query = <QueryParams>{};
        if (filter.page > 1) {
            query.page = filter.page;
        }
        if (filter.categoryId > 0) {
            query.cat = filter.categoryId;
        }
        if (filter.couponId > 0) {
            query.couponId = filter.couponId;
        }
        if (filter.brandIds.length > 0) {
            query.brand = filter.brandIds.join(",");
        }
        if (filter.sortField != "" && ["sale", "price", "time"].includes(filter.sortField)) {
            query.sort = filter.sortField;
        }
        if (filter.sortOrder != "" && ["desc", "asc"].includes(filter.sortOrder)) {
            query.order = filter.sortOrder;
        }
        if (filter.maxPrice != null && filter.maxPrice > 0) {
            query.max = filter.maxPrice;
        }
        if (filter.minPrice != null && filter.minPrice > 0) {
            query.min = filter.minPrice;
        }
        if (filter.keyword != "") {
            query.keyword = filter.keyword;
        }
        const attrsQuery = serializeAttrsQuery(attrList.value);
        if (attrsQuery) {
            query.attrs = attrsQuery;
        }
        return query;
    });

    // ===================== SEO =====================
    const normalizedPagePath = computed(() => (pageType.value.startsWith("/") ? pageType.value : `/${pageType.value}`));
    const currentCategoryName = computed(() => String(filterSeleted.value?.category || ""));
    const currentBrandName = computed(() => String(filterSeleted.value?.brand || ""));
    const canonical = computed(() => {
        if (pageType.value === "search") {
            return filter.keyword ? `/search/?keyword=${encodeURIComponent(filter.keyword)}` : "/search/";
        }
        const params = new URLSearchParams();
        if (filter.categoryId > 0) {
            params.set("cat", String(filter.categoryId));
        }
        if (filter.couponId > 0) {
            params.set("couponId", String(filter.couponId));
        }
        return params.size > 0 ? `${normalizedPagePath.value}/?${params.toString()}` : `${normalizedPagePath.value}/`;
    });

    useSeo({
        title: computed(() => {
            if (pageType.value === "search" && filter.keyword) {
                return `${t("搜索")}「${filter.keyword}」`;
            }
            if (currentCategoryName.value) {
                return currentCategoryName.value;
            }
            return t("商品列表");
        }),
        description: computed(() => {
            if (pageType.value === "search" && filter.keyword) {
                return `${t("搜索结果")}：${filter.keyword}`;
            }
            if (currentCategoryName.value) {
                return `${t("浏览")}${currentCategoryName.value}${t("商品")}`;
            }
            return commonStore.shopDesc || "";
        }),
        keywords: computed(() =>
            buildSeoKeywords([filter.keyword, currentCategoryName.value, currentBrandName.value, commonStore.shopName, commonStore.shopTitle])
        ),
        canonical,
        robots: computed(() => (pageType.value === "search" ? "noindex,follow" : "index,follow"))
    });

    // ===================== 行为方法 =====================
    /** 筛选条件变更：重置页码并把 queryParams 写入 URL */
    const onFilterChange = (page: number = 1): void => {
        filter.page = page;
        navigateTo({
            path: "/" + pageType.value + "/",
            query: queryParams.value
        });
    };

    /** 属性筛选变更回调，更新属性列表 */
    const onFilterAttrChange = (attrs: any[]): void => {
        attrList.value = attrs;
    };

    /** 清除指定属性的筛选条件 */
    const clearAttr = (name: string): void => {
        attrList.value = attrList.value.filter((item) => item.attrName !== name);
        onFilterChange();
    };

    /** 逐级撤销最后一步筛选 */
    const onFilterUndo = (): void => {
        if (attrList.value.length > 0) {
            attrList.value = undoLastSelectedAttr(attrList.value);
            onFilterChange();
            return;
        }
        if (filter.brandIds.length > 0) {
            filter.brandIds = filter.brandIds.slice(0, -1);
            onFilterChange();
            return;
        }
        const hasPrice = Number(filter.maxPrice) > 0 || (filter.minPrice != null && Number(filter.minPrice) > 0);
        if (hasPrice) {
            filter.maxPrice = null;
            filter.minPrice = null;
            onFilterChange();
            return;
        }
        if (filter.keyword) {
            filter.keyword = "";
            onFilterChange();
            return;
        }
        if (filter.categoryId > 0) {
            filter.categoryId = 0;
            onFilterChange();
            return;
        }
        if (typeof window !== "undefined" && window.history.length > 1) {
            router.back();
            return;
        }
        navigateTo({
            path: "/" + pageType.value + "/",
            query: {}
        });
    };

    return {
        pageType,
        filter,
        filterSeleted,
        attrList,
        queryParams,
        onFilterChange,
        onFilterAttrChange,
        onFilterUndo,
        clearAttr
    };
}

export function useSearchPageContext(): SearchPageContext {
    const ctx = inject(SearchPageInjectionKey);
    if (!ctx) {
        throw new Error("[useSearchPageContext] 必须在 search 容器组件内使用");
    }
    return ctx;
}
