<template>
    <div class="search-page" :class="searchStyleClass" :style="searchCssVars">
        <CommonHeader :title="pageTitle"></CommonHeader>
        <div data-content-rev="YXVzczIyODI" class="page-body">
            <slot name="top"></slot>
            <SearchLayoutSide
                v-if="isFilterSide"
                v-model:filter="filter"
                v-model:filter-seleted="filterSeleted"
                v-model:filter-loading="FilterLoading"
                :query-params="requestQueryParams"
                :search-config="searchConfig as any"
                :page-type="pageType"
                @change="onFilterChange"
                @list-length="listLength"
                @load-finished="onGoodsInitialLoadFinished"
            />
            <div v-else class="container" :class="{ 'container--initial-loading': showInitialSkeleton }">
                <div class="top-filter-wrap">
                    <Filter
                        v-if="activeContentTab === 'goods'"
                        ref="filterRef"
                        v-model:filter-loading="FilterLoading"
                        v-model:filter="filter"
                        v-model:filter-seleted="filterSeleted"
                        :query-params="requestQueryParams"
                        :keyword="filter.keyword"
                        :filter-style="searchConfig.filterStyle"
                        :page-type="pageType"
                        @change="onFilterChange"
                        @brand-options-change="onBrandOptionsChange"
                        @panel-data-change="onPanelDataChange"
                    />
                    <div class="search-tabs">
                        <div class="search-tabs__left">
                            <div
                                v-for="item in searchTabs"
                                :key="item.value"
                                class="search-tabs__item"
                                :class="{ active: activeContentTab === item.value }"
                                @click="activeContentTab = item.value"
                            >
                                {{ $t(item.label) }}
                            </div>
                        </div>
                        <div v-if="activeContentTab === 'goods' && searchStore.total > 0" class="search-tabs__right">
                            <div class="pageInfo">
                                <span class="noPage first-page" :class="{ disabled: filter.page == 1 }" @click="changeTopPage(-1)">
                                    <tig-icon name="arrow" :size="16" class="page-arrow page-arrow--prev" />
                                </span>
                                <span class="page_number">
                                    <em class="page_current">{{ filter.page }}</em>
                                    <span class="page_separator">/</span>
                                    <span class="page_total">{{ pageCount }}</span>
                                </span>
                                <span class="noPage" :class="{ disabled: filter.page >= pageCount }" @click="changeTopPage(1)">
                                    <tig-icon name="arrow" :size="16" class="page-arrow" />
                                </span>
                            </div>
                        </div>
                    </div>
                </div>
                <div class="right-wrap">
                    <template v-if="activeContentTab === 'goods'">
                        <SortFilter
                            v-model:filter="filter"
                            :filter-style="searchConfig.filterStyle"
                            :search-config="searchConfig as any"
                            :show-price-range="searchConfig.showPriceRange !== false"
                            :total="searchStore.total"
                            :page-count="pageCount"
                            :show-page-switcher="false"
                            :brand-options="brandOptions"
                            :filter-panel-data="filterPanelData"
                            @change="onFilterChange"
                            @panel-visible-change="onFilterPanelVisibleChange"
                        />
                        <div class="goods-list-wrap" :class="{ 'goods-list-wrap--filter-panel-open': filterPanelVisible }">
                            <CatGoodsList
                                ref="productRef"
                                v-model:filter-loading="FilterLoading"
                                v-model:filter="filter"
                                :query-params="requestQueryParams"
                                :filter-seleted="filterSeleted"
                                :search-config="searchConfig as any"
                                :fixed-name-two-lines="true"
                                @change="onFilterChange"
                                @list-length="listLength"
                                @load-finished="onGoodsInitialLoadFinished"
                            />
                        </div>
                    </template>
                    <SearchShopList v-else :active="activeContentTab === 'shop'" :keyword="filter.keyword" />
                </div>
                <SearchSkeleton v-if="showInitialSkeleton" />
            </div>
        </div>
        <!-- 替代 PageFooter mt30：实体高度占位，沿用搜索页底色，避免 margin 透出外层色带 -->
        <div class="search-footer-spacer" aria-hidden="true"></div>
    </div>
</template>
<script lang="ts" setup>
// ===================== Vue 核心 =====================
import { ref, computed, watch } from "vue";

// ===================== 组件导入 =====================
import SortFilter from "./SortFilter.vue";
import Filter from "./Filter.vue";
import CatGoodsList from "./CatGoodsList.vue";
import SearchLayoutSide from "./SearchLayoutSide.vue";
import SearchSkeleton from "./SearchSkeleton.vue";
import SearchShopList from "./SearchShopList.vue";

// ===================== API 导入 =====================
import { decorateApi } from "./api";

// ===================== Store 导入 =====================
import { useDecorateRuntimeStore, type PcSearchModuleData } from "~/store/decorateRuntime";
import { useSearchStore } from "~/store/search";
import { resolvePcSearchModuleData, type SearchDecorateResult } from "~/utils/resolvePcSearchModuleData";

// ===================== 类型导入 =====================
import type { filterSeleted, SearchFilter, QueryParams, SearchFilterListResult } from "./types.d";

// ===================== 工具函数 =====================
import { isShop, isO2o, isOverseas } from "~/utils/util";

// ===================== 组合式函数 =====================
const { t } = useI18n();
// ===================== Props & Emits =====================
const props = defineProps({
    pageType: {
        type: String,
        default: "/search"
    },
    /** 图搜编号：传入后列表、筛选项都只在本次图片召回的商品内查询 */
    searchToken: {
        type: String,
        default: ""
    }
});

// ===================== Store 实例 =====================
const decorateRuntimeStore = useDecorateRuntimeStore();
const searchStore = useSearchStore();
const route = useRoute();
// 口径与首页一致：非法或重复的 previewId 一律按非预览态处理，避免 String(["1","2"]) 拼出 "1,2"
const previewId = computed(() => {
    const id = Number(route.query.previewId) || 0;
    return id > 0 ? String(id) : "";
});

// ===================== 装修配置 =====================
type SearchFilterAttrOption = {
    value: string;
    checked?: boolean;
};

type SearchFilterAttrItem = {
    name: string;
    values: SearchFilterAttrOption[];
};

type SearchFilterPanelData = {
    brand: SearchFilterListResult["brand"];
    category: SearchFilterListResult["category"];
    attrs: SearchFilterAttrItem[];
};

// ===================== SSR 搜索装修数据获取 =====================
// 对齐登录页：统一走 home/home/pcSearch，previewId > 0 时拉草稿/指定模板
const { data: searchDecorate } = await useAsyncData(
    "search-decorate",
    async (): Promise<SearchDecorateResult | null> => {
        // 预览态必须请求接口，不能被 store 缓存短路
        if (!previewId.value && decorateRuntimeStore.pcSearchConfig) return null;
        // home/home/pcSearch：request 已解包为 { decorateId, moduleList, pageModule }
        return (await decorateApi.getPcSearch(
            previewId.value ? { previewId: previewId.value } : undefined
        )) as SearchDecorateResult | null;
    },
    { watch: [previewId], default: () => null }
);

/** 搜索装修配置 */
const searchConfig = computed<PcSearchModuleData>(() => {
    const normalizeBoolLike = (val: unknown, defaultVal: boolean): boolean => {
        if (val === undefined || val === null) return defaultVal;
        // 后台装修常见下发：0/1 或 "0"/"1"；也可能是 "true"/"false"
        if (val === false || val === 0 || val === "0" || val === "false") return false;
        if (val === true || val === 1 || val === "1" || val === "true") return true;
        return Boolean(val);
    };
    const normalizeNumberLike = (val: unknown, defaultVal: number): number => {
        if (val === undefined || val === null || val === "") return defaultVal;
        const num = Number(val);
        return Number.isFinite(num) ? num : defaultVal;
    };
    const normalizeThemeColor = (val: unknown): string => {
        return typeof val === "string" && val.trim() ? val.trim() : "var(--general)";
    };

    // 默认顶部筛选（与后台装修默认一致）
    const defaultConfig: PcSearchModuleData = {
        backgroundColor: "#fff",
        themeColor: "#ff4d4f",
        filterStyle: 2,
        filterTemplate: 2,
        showFilterPanel: true,
        showPriceRange: true,
        showShopTag: true,
        showSales: true,
        showCartButton: true,
        goodsStyle: 1,
        goodsRadiusStyle: 1,
        buyStyle: 1,
        componentMarginTop: 8,
        componentMarginBottom: 8
    };
    // 预览态优先用接口数据；正式态才允许 store 缓存覆盖
    const storeConfig = previewId.value ? null : decorateRuntimeStore.pcSearchConfig;
    const moduleData = resolvePcSearchModuleData(searchDecorate.value as SearchDecorateResult | null);
    const rawConfig: PcSearchModuleData | null = storeConfig
        ? ({ ...defaultConfig, ...storeConfig } as PcSearchModuleData)
        : moduleData ? { ...defaultConfig, ...moduleData } : null;

    if (!rawConfig) {
        return {
            ...defaultConfig,
            themeColor: normalizeThemeColor(defaultConfig.themeColor)
        };
    }

    // 接口常把数字/布尔序列化成字符串，这里统一规范
    return {
        ...rawConfig,
        themeColor: normalizeThemeColor(rawConfig.themeColor),
        filterStyle: normalizeNumberLike(rawConfig.filterStyle, defaultConfig.filterStyle ?? 2) as 1 | 2,
        filterTemplate: normalizeNumberLike(rawConfig.filterTemplate, defaultConfig.filterTemplate ?? 2),
        goodsStyle: normalizeNumberLike(rawConfig.goodsStyle, defaultConfig.goodsStyle ?? 1),
        goodsRadiusStyle: normalizeNumberLike(rawConfig.goodsRadiusStyle, defaultConfig.goodsRadiusStyle ?? 1),
        buyStyle: normalizeNumberLike(rawConfig.buyStyle, defaultConfig.buyStyle ?? 1),
        componentMarginTop: normalizeNumberLike(rawConfig.componentMarginTop, defaultConfig.componentMarginTop ?? 8),
        componentMarginBottom: normalizeNumberLike(rawConfig.componentMarginBottom, defaultConfig.componentMarginBottom ?? 8),
        showFilterPanel: normalizeBoolLike(rawConfig.showFilterPanel, defaultConfig.showFilterPanel ?? true),
        showPriceRange: normalizeBoolLike(rawConfig.showPriceRange, defaultConfig.showPriceRange ?? true),
        showShopTag: normalizeBoolLike(rawConfig.showShopTag, defaultConfig.showShopTag ?? true),
        showSales: normalizeBoolLike(rawConfig.showSales, defaultConfig.showSales ?? true),
        showCartButton: normalizeBoolLike(rawConfig.showCartButton, defaultConfig.showCartButton ?? true)
    };
});

/** CSS 变量 */
const searchCssVars = computed(() => {
    const vars: Record<string, string> = {};
    vars["--search-bg-color"] = searchConfig.value.backgroundColor || "#fff";
    vars["--search-margin-top"] = `${searchConfig.value.componentMarginTop ?? 8}px`;
    // vars["--search-theme-color"] = searchConfig.value.themeColor || "var(--general)";
    if (searchConfig.value.componentMarginBottom !== undefined) {
        vars["--search-margin-bottom"] = `${searchConfig.value.componentMarginBottom}px`;
    }
    const filterTemplate = Number(searchConfig.value.filterTemplate ?? 1);
    const baseGridColumns = filterTemplate === 2 ? 6 : 5;
    const currentGridColumns = filterPanelVisible.value && baseGridColumns === 6 ? 5 : baseGridColumns;
    vars["--search-grid-columns"] = String(currentGridColumns);
    // 商品圆角
    const radiusMap: Record<number, string> = { 1: "0px", 2: "8px", 3: "16px" };
    vars["--search-item-radius"] = radiusMap[searchConfig.value.goodsRadiusStyle ?? 1] || "0px";
    return vars;
});

/** 是否为左侧筛选布局（filterStyle=1） */
const isFilterSide = computed(() => Number(searchConfig.value.filterStyle ?? 2) === 1);

/** 商品样式 class */
const searchStyleClass = computed(() => {
    const classes: string[] = [];
    if (Number(searchConfig.value.filterStyle ?? 2) === 2) {
        classes.push("search-page--filter-top");
    }
    // 商品样式
    const styleMap: Record<number, string> = { 1: "search-page--goods-flat", 2: "search-page--goods-card", 3: "search-page--goods-outline" };
    classes.push(styleMap[searchConfig.value.goodsStyle ?? 1] || "search-page--goods-flat");
    // 圆角
    const radiusMap: Record<number, string> = { 1: "search-page--radius-0", 2: "search-page--radius-8", 3: "search-page--radius-16" };
    classes.push(radiusMap[searchConfig.value.goodsRadiusStyle ?? 1] || "search-page--radius-0");
    return classes;
});

const pageCount = computed(() => Math.max(1, Math.ceil((searchStore.total || 0) / 30)));
const pageTitle = computed(() => {
    if (props.searchToken) return t("图片搜索");
    return filter.keyword ? `${filter.keyword} - ${t("商品搜索")}` : t("商品搜索");
});
const shopEnabled = computed(() => isShop() && !isO2o() && !isOverseas());
const showInitialSkeleton = computed(() => activeContentTab.value === "goods" && !initialGoodsLoaded.value);
const searchTabs = computed(() => {
    const tabs: Array<{ label: string; value: "goods" | "shop" }> = [{ label: t("全部商品"), value: "goods" }];
    if (shopEnabled.value) {
        tabs.push({ label: t("店铺"), value: "shop" });
    }
    return tabs;
});

// 装修数据通过 useAsyncData 获取，无需手动 watch

// ===================== 路由 =====================
// route 已在上方初始化（用于 previewId 和 query 读取）

// ===================== 响应式状态 =====================
const filterRef = ref();
const filter = reactive<SearchFilter>({
    page: 1,
    categoryId: "",
    pId: "",
    brandIds: [],
    attrs: [],
    sortField: "",
    sortOrder: "",
    maxPrice: 0,
    minPrice: 0,
    couponId: "",
    keyword: ""
});
const FilterLoading = ref(false);
const cat = defineModel("cat", {
    type: String,
    default: ""
});
const filterSeleted = ref<filterSeleted>();
const productLength = ref(0);
const activeContentTab = ref<"goods" | "shop">("goods");
const filterPanelVisible = ref(false);
const initialGoodsLoaded = ref(false);
const brandOptions = ref<SearchFilterListResult["brand"]>([]);
const filterPanelData = ref<SearchFilterPanelData>({
    brand: [],
    category: [],
    attrs: []
});

// ===================== 计算属性 =====================
/** URL / 请求上的 sort 归一化为 filter.sortField（兼容历史错误值 productPrice） */
const normalizeSortField = (sort: unknown): string => {
    const value = sort ? String(sort) : "";
    if (value === "productPrice") return "price";
    if (["sale", "price", "time"].includes(value)) return value;
    return "";
};

const buildBaseQuery = (source: {
    couponId?: unknown;
    keyword?: unknown;
    cat?: unknown;
    pId?: unknown;
    page?: unknown;
    brand?: unknown;
    sort?: unknown;
    order?: unknown;
    min?: unknown;
    max?: unknown;
}): QueryParams => {
    const query: QueryParams = {};
    if (source.couponId) {
        query.couponId = String(source.couponId);
    }
    if (source.keyword) {
        query.keyword = String(source.keyword);
    }
    if (source.cat) {
        query.cat = String(source.cat);
    }
    if (source.pId) {
        query.pId = String(source.pId);
    }
    if (source.brand) {
        query.brand = String(source.brand);
    }
    const page = Number(source.page);
    if (Number.isFinite(page) && page > 1) {
        query.page = page;
    }
    // 后端 ProductSearchService 只认 sale|price|time（不是 productPrice）
    const sortField = normalizeSortField(source.sort);
    if (sortField) {
        query.sort = sortField;
    }
    const order = source.order ? String(source.order) : "";
    if (order && ["desc", "asc"].includes(order)) {
        query.order = order;
    }
    const min = Number(source.min);
    if (Number.isFinite(min) && min > 0) {
        query.min = min;
    }
    const max = Number(source.max);
    if (Number.isFinite(max) && max > 0) {
        query.max = max;
    }
    return query;
};

const nextRouteQueryParams = computed(() =>
    buildBaseQuery({
        couponId: filter.couponId,
        keyword: filter.keyword,
        cat: filter.categoryId || route.query.cat,
        pId: filter.pId || route.query.pId,
        page: filter.page,
        brand: filter.brandIds.length > 0 ? filter.brandIds.join(",") : "",
        sort: filter.sortField,
        order: filter.sortOrder,
        min: filter.minPrice,
        max: filter.maxPrice
    })
);

const requestQueryParams = computed(() => {
    const query: QueryParams = { ...nextRouteQueryParams.value };
    if (filter.categoryId) {
        query.cat = filter.categoryId;
    }
    if (filter.pId) {
        query.pId = filter.pId;
    }
    if (filter.brandIds.length > 0) {
        query.brand = filter.brandIds.join(",");
    }
    if (filter.page > 1) {
        query.page = filter.page;
    }
    // 与跨境 / 后端一致：sort 传 price|sale|time
    if (filter.sortField != "" && ["sale", "price", "time"].includes(filter.sortField)) {
        query.sort = filter.sortField;
    }
    if (filter.sortOrder != "" && ["desc", "asc"].includes(filter.sortOrder)) {
        query.order = filter.sortOrder;
    }
    if (filter.maxPrice > 0) {
        query.max = filter.maxPrice;
    }
    if (filter.minPrice > 0) {
        query.min = filter.minPrice;
    }
    if (props.searchToken) {
        query.searchToken = props.searchToken;
    }
    return query;
});

// ===================== 事件处理方法 =====================
const listLength = (length: number): void => {
    productLength.value = length;
};

const onBrandOptionsChange = (list: SearchFilterListResult["brand"]): void => {
    brandOptions.value = list || [];
};

const onPanelDataChange = (data: SearchFilterPanelData): void => {
    filterPanelData.value = {
        brand: data?.brand || [],
        category: data?.category || [],
        attrs: data?.attrs || []
    };
};

const onFilterPanelVisibleChange = (visible: boolean): void => {
    filterPanelVisible.value = visible;
};

const onGoodsInitialLoadFinished = (): void => {
    initialGoodsLoaded.value = true;
};

const changeTopPage = (step: number): void => {
    const nextPage = filter.page + step;
    if (nextPage < 1 || nextPage > pageCount.value) return;
    onFilterChange(nextPage);
};

const onFilterChange = (page: number = 1): void => {
    // 对齐跨境：每次筛选/排序变更都写回 URL（推荐=清空 sort/order），并默认回到第 1 页
    filter.page = page;
    const query: QueryParams = { ...nextRouteQueryParams.value };
    // 筛选/排序写回 URL 时保留装修预览参数，避免预览态被冲掉（不带入商品列表请求）
    if (previewId.value) {
        query.previewId = previewId.value;
    }
    if (props.searchToken) {
        query.searchToken = props.searchToken;
    }
    navigateTo({
        path: props.pageType,
        query
    });
};
// ===================== 副作用 =====================
watch(
    () => [
        route.query.keyword,
        route.query.couponId,
        route.query.cat,
        route.query.pId,
        route.query.page,
        route.query.brand,
        route.query.sort,
        route.query.order,
        route.query.min,
        route.query.max
    ],
    (newValues, oldValues) => {
        const [nextKeyword, nextCouponId, nextCat, nextPId, nextPage, nextBrand, nextSort, nextOrder, nextMin, nextMax] = newValues;
        const isInitial = !oldValues;
        // keyword/coupon/cat/pId 变化时重置排序与价格；sort/order/brand/page 变化不重置
        const shouldResetFilterState = !isInitial && newValues.slice(0, 4).some((item, index) => item !== oldValues![index]);
        const parsedPage = Number(nextPage);

        filter.page = Number.isFinite(parsedPage) && parsedPage > 0 ? parsedPage : 1;

        if (shouldResetFilterState) {
            filter.sortField = "";
            filter.sortOrder = "";
            filter.maxPrice = 0;
            filter.minPrice = 0;
        } else {
            // 首次进入或仅排序/价格变化：从 URL 同步
            filter.sortField = normalizeSortField(nextSort);
            const order = nextOrder ? String(nextOrder) : "";
            filter.sortOrder = ["desc", "asc"].includes(order) ? order : "";
            filter.minPrice = nextMin ? Number(nextMin) : 0;
            filter.maxPrice = nextMax ? Number(nextMax) : 0;
        }

        const brandStr = nextBrand ? String(nextBrand) : "";
        filter.brandIds = brandStr ? brandStr.split(",").filter(Boolean) : [];
        filter.keyword = nextKeyword ? String(nextKeyword) : "";
        filter.couponId = nextCouponId ? String(nextCouponId) : "";

        if (nextCat) {
            cat.value = String(nextCat);
            filter.categoryId = String(nextCat);
            filter.pId = nextPId ? String(nextPId) : "";
        } else if (shouldResetFilterState || isInitial) {
            cat.value = "";
            filter.categoryId = "";
            filter.pId = "";
        }
    },
    {
        immediate: true
    }
);

// ===================== 监听器 =====================
watch(
    () => filterSeleted.value,
    (newVal) => {
        if (newVal?.brand == null) {
            filterRef.value?.clearMultipleBrandIds();
        }
    }
);

watch(
    () => shopEnabled.value,
    (enabled) => {
        if (!enabled && activeContentTab.value === "shop") {
            activeContentTab.value = "goods";
        }
        if (!enabled) {
            filterPanelVisible.value = false;
        }
    },
    {
        immediate: true
    }
);

watch(activeContentTab, (tab) => {
    if (tab !== "goods") {
        filterPanelVisible.value = false;
    }
});
</script>
<style lang="less" scoped>
.search-page {
    background-color: var(--search-bg-color, #fff);
}

.search-footer-spacer {
    height: 30px;
}

.page-body {
    padding-top: var(--search-margin-top, 32px);
    padding-bottom: var(--search-margin-bottom, 0);
}

.container {
    display: flex;
    justify-content: space-between;
    flex-wrap: wrap;
    width: 100%;
    max-width: var(--pc-home-max-width, 1600px);
    min-width: var(--pc-home-min-width, 1264px);
    padding: 0 var(--pc-home-padding-x, 38px);
    box-sizing: border-box;
    position: relative;

    &--filter-top {
        display: block;
    }

    &--initial-loading {
        overflow: hidden;
        min-height: calc(100vh - 120px);

        > :not(.search-page-skeleton) {
            visibility: hidden;
        }
    }

    .top-filter-wrap {
        width: 100%;
    }

    .left-wrap {
        width: 240px;
    }

    .right-wrap {
        width: 100%;

        &--full {
            width: 100%;
        }
    }

    .goods-list-wrap {
        width: 100%;
        box-sizing: border-box;
        transition: padding-right 0.2s ease;

        &--filter-panel-open {
            padding-right: 384px;
        }
    }
    .nav-selected {
        width: 0;
        flex: 1;
        .item {
            max-width: 100%;
            overflow: hidden;
            text-overflow: ellipsis;
            box-sizing: border-box;
        }
    }
}

.search-page-skeleton {
    position: absolute;
    top: 0;
    left: var(--pc-home-padding-x, 38px);
    right: var(--pc-home-padding-x, 38px);
    z-index: 3;
    pointer-events: none;
}

.search-tabs {
    display: flex;
    align-items: center;
    justify-content: space-between;
}

.search-tabs__left {
    display: flex;
    align-items: center;
    gap: 32px;
}

.search-tabs__right {
    display: flex;
    align-items: center;
    flex-shrink: 0;
}

.search-tabs__item {
    position: relative;
    font-size: 16px;
    line-height: 48px;
    color: #1f1f1f;
    cursor: pointer;
    transition: color 0.2s;
    user-select: none;

    &:hover,
    &.active {
        color: var(--general);
        font-weight: 700;
    }

    &.active::after {
        content: "";
        position: absolute;
        left: 0;
        right: 0;
        bottom: 0;
        height: 2px;
        border-radius: 999px;
        background: var(--general);
    }
}

.pageInfo {
    display: flex;
    align-items: center;
    // gap: 5px;

    .page_number {
        color: #111;
        font-size: 16px;
        display: flex;
        align-items: baseline;
        font-family: Arial, sans-serif;
    }

    .page_current {
        color: var(--general);
        font-style: normal;
        font-weight: 500;
    }

    .page_separator {
        margin: 0 2px;
        color: #111;
    }

    .page_total {
        color: #111;
    }

    .noPage {
        width: auto;
        height: auto;
        border: none;
        display: inline-flex;
        align-items: center;
        justify-content: center;
        background: transparent;
        color: #333;
        cursor: pointer;
        line-height: 1;
        padding: 10px 10px 10px 5px;

        .page-arrow {
            transform: rotate(-90deg);
        }

        .page-arrow--prev {
            transform: rotate(90deg);
        }

        &.disabled {
            color: #ccc;
            cursor: not-allowed;
        }
    }
    .first-page {
        padding: 10px 5px 10px 10px;
    }
}
.filter-box {
    width: 220px;
    border: 1px solid #e0e1e5;
    border-radius: 8px;
    padding: 12px;
    align-self: flex-start;
    position: sticky;
    top: 32px;
    max-height: calc(100vh - 64px - 72px);
    overflow: auto;
    font-size: 14px;
    color: #1a1a1a;
    font-weight: bold;
    span {
        color: #bfbfbf;
    }
}

// ========== 商品样式修饰类 ==========
.search-page--goods-flat {
    :deep(.product-card-container) {
        background-color: transparent;
        border: none;
        box-shadow: none;

        &:hover {
            box-shadow: none;
        }
    }
}

.search-page--goods-card {
    :deep(.product-card-container) {
        background-color: #fff;
        border: none;
        box-shadow: 0 2px 8px rgba(0, 0, 0, 0.08);
    }
}

.search-page--goods-outline {
    :deep(.product-card-container) {
        background-color: #fff;
        border: 1px solid var(--search-theme-color, var(--general)) !important;

        &:hover {
            box-shadow: none;
        }
    }
}

// ========== 圆角样式修饰类 ==========
.search-page--radius-0 {
    --search-item-radius: 0px;
}

.search-page--radius-8 {
    --search-item-radius: 8px;
}

.search-page--radius-16 {
    --search-item-radius: 16px;
}

.search-page--filter-top {
    .container {
        display: block;
    }
}

:deep(.product-card-container) {
    border-radius: var(--search-item-radius, 4px);

    .top-wrap .pro_pic {
        border-radius: var(--search-item-radius, 4px);
        overflow: hidden;
    }
}
</style>
