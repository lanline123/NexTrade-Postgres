<template>
    <div class="filter-container filter-container--top">
        <div class="filter-top">
            <div class="filter-top-body">
                <div v-if="(categoryDisplayList ?? []).length > 0" class="list">
                    <div class="attr-box">
                        <div v-if="showCategoryLeftArrow" class="filter-category__arrow filter-category__arrow--left" @click="scrollCategory('left')">
                            <span class="filter-category__arrow-btn">
                                <tig-icon color="#1f1f1f" name="arrow-right" class="filter-category__arrow-icon filter-category__arrow-icon--flip" />
                            </span>
                        </div>
                        <div ref="categoryTabsRef" class="category-link-wrapper" @scroll="updateCategoryArrows">
                            <div ref="categoryLinkRef" class="category-link">
                                <NuxtLink
                                    :to="categoryLink(item)"
                                    v-for="item in categoryDisplayList"
                                    :key="item.id"
                                    :ref="(el) => setCategoryItemRef(el, item.id)"
                                    :aria-current="topCategorySelectedId === String(item.id) ? 'page' : undefined"
                                    class="item"
                                    :class="{
                                        active: topCategorySelectedId === String(item.id),
                                        loading: categoryLoadingId === String(item.id)
                                    }"
                                    @click.capture="handleTopCategoryClick(item.id, $event)"
                                >
                                    <img v-if="item.categoryPic" :src="imageFormat(item.categoryPic)" :alt="item.categoryName" class="category-pic" />
                                    <span class="category-name" :style="[{ paddingLeft: item.categoryPic ? '0' : '12px' }]">{{ item.categoryName }}</span>
                                </NuxtLink>
                            </div>
                        </div>
                        <div v-if="showCategoryRightArrow" class="filter-category__arrow filter-category__arrow--right" @click="scrollCategory('right')">
                            <span class="filter-category__arrow-btn">
                                <tig-icon color="#1f1f1f" name="arrow-right" class="filter-category__arrow-icon" />
                            </span>
                        </div>
                    </div>
                </div>
            </div>
        </div>
    </div>
</template>

<script lang="ts" setup>
// ===================== Vue 核心 =====================
import { computed, nextTick, onBeforeUnmount, onMounted, reactive, ref, shallowRef, watch, watchEffect } from "vue";

// ===================== API 导入 =====================
import { searchApi } from "./api";

// ===================== 类型导入 =====================
import type { ApiSearchCategoryVO, filterSeleted, SearchFilter, SearchFilterBrandRow, ID } from "./types.d";
import type { ComponentPublicInstance, Ref } from "vue";

import { imageFormat, urlFormat } from "~/utils/format";

// ===================== Store =====================
import { useCommonStore } from "~/store/common";
import { useSearchStore } from "~/store/search";

// ===================== 工具函数 =====================
import { deepClone } from "~/utils/util";

// ===================== Props & Emits =====================
const props = defineProps({
    keyword: {
        type: String,
        default: ""
    },
    queryParams: {
        type: Object,
        default: () => ({})
    },
    pageType: {
        type: String,
        default: "search"
    },
    filterStyle: {
        type: Number,
        default: 1
    }
});

const emit = defineEmits(["change", "changeAttr", "brandOptionsChange", "panelDataChange"]);

const route = useRoute();

// ===================== Store 实例 =====================
const commonStore = useCommonStore();
const searchStore = useSearchStore();

// ===================== 响应式状态 =====================
const brandCollapseActive = ref([1]);
const collapseActive = ref([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20]);
const isShowMore = ref(false);
const price = ref([0, 0]);
const categoryShowMore = ref(false);
const categoryNeedMore = ref(false);
const categoryTabsRef = ref<HTMLElement | null>(null);
const categoryLinkRef = ref<HTMLElement | null>(null);
const categoryItemRefs = ref<Record<string, HTMLElement>>({});
const categoryDisplayList = ref<ApiSearchCategoryVO[]>([]);
const categoryLoadingId = ref("");
const topCategorySelectedId = ref("");
const showCategoryLeftArrow = ref(false);
const showCategoryRightArrow = ref(false);
const topBrandShowMore = ref(false);
const sideBrandShowMore = ref(false);
const curPice = ref(0);
const filter = defineModel<SearchFilter>("filter", { default: { brandIds: [], attrs: [] } }) as Ref<SearchFilter>;
const sliderMax = shallowRef<number>(0);
const filterSeleted = defineModel<filterSeleted>("filterSeleted");
const headTitle = ref("");
const _filterSeleted = ref<filterSeleted>({
    category: "",
    brand: "",
    attrs: []
});
type SearchFilterAttrOption = {
    value: string;
    checked?: boolean;
};

type SearchFilterAttrItem = {
    name: string;
    values: SearchFilterAttrOption[];
};

type SearchFilterListState = {
    brand: SearchFilterBrandRow[];
    category: ApiSearchCategoryVO[];
    attrs: SearchFilterAttrItem[];
};

type SearchFilterPanelData = {
    brand: SearchFilterBrandRow[];
    category: ApiSearchCategoryVO[];
    attrs: SearchFilterAttrItem[];
};

const filterList = reactive<SearchFilterListState>({
    brand: [],
    category: [],
    attrs: []
});
const filterLoading = defineModel<boolean>("filterLoading", { default: false });
const topBrandMultiple = ref(false);
const selectedLetter = ref("");
let categoryResizeObserver: ResizeObserver | null = null;

const filteredAlphabets = computed(() => {
    return [...new Set(filterList.brand.map((item) => resolveBrandFirstWord(item)).filter(Boolean))].sort();
});

const filteredBrands = computed(() => {
    if (!selectedLetter.value) return filterList.brand;
    return filterList.brand.filter((item) => resolveBrandFirstWord(item) === selectedLetter.value);
});

const visibleTopBrands = computed(() => {
    return topBrandShowMore.value ? filteredBrands.value : filteredBrands.value.slice(0, 8);
});

const showTopBrandMore = computed(() => {
    return filterList.brand.length > 8;
});

const CATEGORY_COLLAPSED_HEIGHT = 40;

const resolveBrandFirstWord = (brand: SearchFilterBrandRow): string => {
    const rawFirstWord = String(
        (
            brand as SearchFilterBrandRow & {
                initial?: string;
                first_word?: string;
                brandFirstWord?: string;
                letter?: string;
            }
        ).firstWord ??
            (
                brand as SearchFilterBrandRow & {
                    initial?: string;
                    first_word?: string;
                    brandFirstWord?: string;
                    letter?: string;
                }
            ).initial ??
            (
                brand as SearchFilterBrandRow & {
                    initial?: string;
                    first_word?: string;
                    brandFirstWord?: string;
                    letter?: string;
                }
            ).first_word ??
            (
                brand as SearchFilterBrandRow & {
                    initial?: string;
                    first_word?: string;
                    brandFirstWord?: string;
                    letter?: string;
                }
            ).brandFirstWord ??
            (
                brand as SearchFilterBrandRow & {
                    initial?: string;
                    first_word?: string;
                    brandFirstWord?: string;
                    letter?: string;
                }
            ).letter ??
            ""
    )
        .trim()
        .toUpperCase();

    if (rawFirstWord) return rawFirstWord;

    const firstChar = String(brand.brandName || "")
        .trim()
        .charAt(0)
        .toUpperCase();
    return /^[A-Z]$/.test(firstChar) ? firstChar : "";
};

// ===================== 数据获取方法 =====================
const _getSearchFilter = async (_type: string = ""): Promise<void> => {
    if (filterLoading.value) return;
    try {
        filterLoading.value = true;
        const result = await searchApi.getFilter({ ...props.queryParams, page_type: props.pageType });
        console.log(result, "result");
        filterList.brand = resolveBrandList(result.filter) ?? [];
        emit("brandOptionsChange", filterList.brand);
        const resultMaxPrice = Number((result.filter as { maxPrice?: number }).maxPrice ?? 0);
        price.value[0] = route.query.min ? Number(route.query.min) : 0;
        price.value[1] = route.query.max ? Number(route.query.max) : resultMaxPrice;
        if (sliderMax.value < resultMaxPrice) {
            sliderMax.value = resultMaxPrice;
        }

        const categories = resolveCategoryList(result.filter) ?? [];
        filterList.category = categories;
        if (!topCategorySelectedId.value || categoryDisplayList.value.length === 0) {
            categoryDisplayList.value = categories;
        }
        console.log(filterList.category, "filterList.category");
        _filterSeleted.value = (result.filterSelected ?? {}) as filterSeleted;
        headTitle.value = "";
        if (_filterSeleted.value.category) {
            headTitle.value += _filterSeleted.value.category + " - ";
        }
        if (filter.value.keyword) {
            headTitle.value += filter.value.keyword + " -  ";
        }
        if (_filterSeleted.value.brand) {
            headTitle.value += _filterSeleted.value.brand + " -  ";
        }
        filterSeleted.value = (result.filterSelected ?? {}) as filterSeleted;
        commonStore.searchCate = categories as typeof commonStore.searchCate;
        emitPanelDataChange();
    } catch (error) {
        console.error("[Filter._getSearchFilter] 获取搜索过滤器失败:", error);
    } finally {
        filterLoading.value = false;
    }
};

const resolveBrandList = (filterData: unknown): SearchFilterBrandRow[] => {
    const data = (filterData || {}) as Record<string, unknown>;
    const rawList = (Array.isArray(data.brand) && data.brand) || (Array.isArray(data.brands) && data.brands) || [];

    return rawList
        .map((item) => {
            const row = (item || {}) as Record<string, unknown>;
            return {
                id: String(row.id ?? row.brandId ?? ""),
                brandName: String(row.brandName ?? row.name ?? ""),
                brandLogo: String(row.brandLogo ?? row.logo ?? ""),
                firstWord: String(row.firstWord ?? row.initial ?? row.first_word ?? row.brandFirstWord ?? row.letter ?? "")
            };
        })
        .filter((item) => item.id && item.brandName);
};

const resolveCategoryList = (filterData: unknown): ApiSearchCategoryVO[] => {
    const data = (filterData || {}) as Record<string, unknown>;
    const rawList =
        (Array.isArray(data.category) && data.category) ||
        (Array.isArray(data.categories) && data.categories) ||
        (Array.isArray(data.shopCategory) && data.shopCategory) ||
        [];

    return rawList
        .map((item) => {
            const row = (item || {}) as Record<string, unknown>;
            const id = String(row.id ?? row.categoryId ?? "");
            const categoryName = String(row.categoryName ?? row.name ?? "");
            const categoryPic = String(row.categoryPic ?? row.pic ?? row.image ?? row.icon ?? "");
            const seoLink = typeof row.seoLink === "string" ? row.seoLink : undefined;
            return { id, categoryName, categoryPic, seoLink };
        })
        .filter((item) => item.id && item.categoryName);
};

const emitPanelDataChange = (): void => {
    emit("panelDataChange", {
        brand: filterList.brand,
        category: filterList.category,
        attrs: filterList.attrs
    } satisfies SearchFilterPanelData);
};
// ===================== 响应式状态（补充） =====================
const multipleBrandIds = ref<string[]>([]);

// ===================== 事件处理方法 =====================
const updateCategoryNeedMore = (): void => {
    const el = categoryLinkRef.value;
    if (!el) return;
    const need = el.scrollHeight > CATEGORY_COLLAPSED_HEIGHT + 1;
    categoryNeedMore.value = need;
    if (!need) {
        categoryShowMore.value = false;
    }
};

const updateCategoryArrows = (): void => {
    const el = categoryTabsRef.value;
    if (!el) {
        showCategoryLeftArrow.value = false;
        showCategoryRightArrow.value = false;
        return;
    }
    showCategoryLeftArrow.value = el.scrollLeft > 0;
    showCategoryRightArrow.value = el.scrollLeft + el.clientWidth < el.scrollWidth - 1;
};

const scrollCategory = (direction: "left" | "right"): void => {
    const el = categoryTabsRef.value;
    if (!el) return;
    const step = el.clientWidth * 0.6;
    el.scrollBy({
        left: direction === "left" ? -step : step,
        behavior: "smooth"
    });
};

const setCategoryItemRef = (el: Element | ComponentPublicInstance | null, categoryId: ID | undefined): void => {
    const key = String(categoryId ?? "");
    if (!key) return;
    const rawEl = el && typeof el === "object" && "$el" in el ? (el as ComponentPublicInstance).$el : (el as HTMLElement | null);
    if (rawEl instanceof HTMLElement) {
        categoryItemRefs.value[key] = rawEl;
        return;
    }
    Reflect.deleteProperty(categoryItemRefs.value, key);
};

const scrollCategoryItemToCenter = async (categoryId: string): Promise<void> => {
    const key = String(categoryId || "");
    if (!key) return;
    await nextTick();
    const container = categoryTabsRef.value;
    const itemEl = categoryItemRefs.value[key];
    if (!container || !itemEl) return;

    const containerRect = container.getBoundingClientRect();
    const itemRect = itemEl.getBoundingClientRect();
    const itemCenter = itemRect.left - containerRect.left + itemRect.width / 2;
    const targetLeft = container.scrollLeft + itemCenter - container.clientWidth / 2;
    const maxScrollLeft = Math.max(container.scrollWidth - container.clientWidth, 0);

    container.scrollTo({
        left: Math.min(Math.max(targetLeft, 0), maxScrollLeft),
        behavior: "smooth"
    });
};

const toggleTopCategoryExpand = (): void => {
    categoryShowMore.value = !categoryShowMore.value;
};

const toggleBrandMultiple = (): void => {
    topBrandMultiple.value = true;
    topBrandShowMore.value = true;
};

const toggleTopBrandExpand = (): void => {
    topBrandShowMore.value = !topBrandShowMore.value;
    if (!topBrandShowMore.value) {
        selectedLetter.value = "";
    }
};

const confirmTopBrandMultiple = (): void => {
    if (multipleBrandIds.value.length === 0) return;
    topBrandMultiple.value = false;
    topBrandShowMore.value = false;
    selectedLetter.value = "";
    selectBrand();
};

const cancelTopBrandMultiple = (): void => {
    topBrandMultiple.value = false;
    topBrandShowMore.value = false;
    selectedLetter.value = "";
    multipleBrandIds.value = [...filter.value.brandIds];
};

/**
 * 顶部分类项的可爬取链接。
 * 普通点击仍由 handleTopCategoryClick 处理筛选，修饰键/中键则可直接打开该 SEO 链接。
 */
const categoryLink = (category: ApiSearchCategoryVO): string => {
    return urlFormat({ path: "list", seoLink: category.seoLink, id: category.id });
};

const filterBrands = (letter: string): void => {
    selectedLetter.value = letter;
};

const selectBrand = (): void => {
    filter.value.brandIds = multipleBrandIds.value;
    filter.value.page = 1;
    emit("change");
};

const handleTopCategoryClick = (categoryId: ID | undefined, event?: MouseEvent): void => {
    // 保留筛选控件的普通点击行为，同时让 Ctrl/Cmd/Shift/Alt + 点击及中键使用真实 SEO 链接。
    if (event && (event.button !== 0 || event.metaKey || event.ctrlKey || event.shiftKey || event.altKey)) {
        return;
    }
    event?.preventDefault();
    const nextCategoryId = String(categoryId ?? "");
    if (!nextCategoryId || categoryLoadingId.value) return;
    const willCancel = topCategorySelectedId.value === nextCategoryId;
    if (!willCancel) {
        void scrollCategoryItemToCenter(nextCategoryId);
    }
    categoryLoadingId.value = nextCategoryId;
    topCategorySelectedId.value = willCancel ? "" : nextCategoryId;
    filter.value.categoryId = willCancel ? "" : nextCategoryId;
    filter.value.pId = "";
    filter.value.page = 1;
    emit("change");
};

const handleTopBrandClick = (brandId: string): void => {
    if (topBrandMultiple.value) {
        const idx = multipleBrandIds.value.indexOf(brandId);
        if (idx > -1) {
            multipleBrandIds.value.splice(idx, 1);
        } else {
            multipleBrandIds.value.push(brandId);
        }
        return;
    }

    multipleBrandIds.value = [brandId];
    selectBrand();
};
const changeAttr = (): void => {
    const attrs = deepClone(filterList.attrs) as SearchFilterAttrItem[];
    const selectedAttrs = attrs
        .map((attr) => {
            // 获取所有被选中的值，不修改原始数组
            const selectedValues = attr.values.filter((item) => item.checked).map((item) => item.value);

            // 如果有选中的值，则返回该属性对象
            if (selectedValues.length > 0) {
                return {
                    attrName: attr.name,
                    attrValue: [...selectedValues] // 创建新的数组副本
                };
            }
            return null;
        })
        .filter((item): item is { attrName: string; attrValue: string[] } => Boolean(item)); // 过滤掉 null 值
    filter.value.attrs = selectedAttrs as unknown as SearchFilter["attrs"];
};

// ===================== 工具方法 =====================
const brandIsSeleted = (brandId: string): boolean => {
    const index = multipleBrandIds.value.indexOf(brandId);
    return index > -1;
};

const onPriceChange = (): void => {
    // if (minPrice.value > maxPrice.value || maxPrice.value === 0) {
    //     return message.error(t("请输入正确的价格范围"));
    // }
    filter.value.minPrice = price.value[0] ?? 0;
    filter.value.maxPrice = price.value[1] ?? 0;
    emit("change");
    // _getSearchFilter();
};

const handlePriceChange = (value: number | number[]): void => {
    if (Array.isArray(value)) {
        if (price.value[0] === value[0] && price.value[1] === value[1]) return;
        price.value = [...value];
        curPice.value = value[1] ?? 0;
    }
};

const _handleRest = (): void => {
    multipleBrandIds.value = [];
    filter.value.brandIds = [];
    filter.value.page = 1;
    emit("change");
};

// ===================== 副作用 =====================
watchEffect(() => {
    curPice.value = route.query.max ? Number(route.query.max) : 0;
    multipleBrandIds.value = [...filter.value.brandIds];
});

watch(
    () => [props.queryParams, props.pageType],
    () => {
        _getSearchFilter();
    },
    { deep: true, immediate: true }
);

watch(
    () => categoryDisplayList.value,
    async () => {
        categoryItemRefs.value = {};
        await nextTick();
        updateCategoryNeedMore();
        updateCategoryArrows();
        if (topCategorySelectedId.value) {
            void scrollCategoryItemToCenter(topCategorySelectedId.value);
        }
    },
    { deep: true }
);

watch(
    () => [props.queryParams, filterLoading.value],
    () => {
        if (!filterLoading.value) {
            categoryLoadingId.value = "";
        }
    },
    { deep: true }
);

watch(topBrandShowMore, (value) => {
    if (!value && !topBrandMultiple.value) {
        selectedLetter.value = "";
    }
});

onMounted(async () => {
    await nextTick();
    updateCategoryNeedMore();
    updateCategoryArrows();

    const el = categoryTabsRef.value;
    if (el && typeof ResizeObserver !== "undefined") {
        categoryResizeObserver = new ResizeObserver(() => {
            updateCategoryNeedMore();
            updateCategoryArrows();
        });
        categoryResizeObserver.observe(el);
    } else if (typeof window !== "undefined") {
        window.addEventListener("resize", updateCategoryNeedMore);
        window.addEventListener("resize", updateCategoryArrows);
    }
});

onBeforeUnmount(() => {
    if (categoryResizeObserver) {
        categoryResizeObserver.disconnect();
        categoryResizeObserver = null;
    }
    if (typeof window !== "undefined") {
        window.removeEventListener("resize", updateCategoryNeedMore);
        window.removeEventListener("resize", updateCategoryArrows);
    }
});

// ===================== 数据获取方法（补充） =====================
const _getSearchESFilter = async (): Promise<void> => {
    try {
        const result = (await searchApi.getEsFilter({
            keyword: props.queryParams.keyword,
            cat: props.queryParams.cat,
            searchToken: props.queryParams.searchToken
        })) as {
            attributes?: SearchFilterAttrItem[];
        };
        result.attributes?.forEach((item: SearchFilterAttrItem) => {
            item.values.forEach((attr: SearchFilterAttrOption) => {
                attr.checked = false;
            });
        });
        filterList.attrs = result.attributes ?? [];
        emitPanelDataChange();
    } catch (error) {
        console.error("获取筛选项失败:", error);
    }
};

// ===================== 监听器 =====================
watch(
    () => [filter.value.keyword, props.queryParams.searchToken],
    () => {
        _getSearchESFilter();
    },
    { deep: true, immediate: true }
);

defineExpose({
    clearMultipleBrandIds() {
        multipleBrandIds.value = [];
    }
});
</script>

<style lang="less" scoped>
.filter-container--top {
    width: 100%;
    padding: 0;
    position: static;
    max-height: none;
    overflow: visible;
    border-radius: 0;
    background: transparent;

    .filter-top {
        // border: 1px solid #e6e6e6;
        background: #fff;
    }

    .sear-title {
        background: #f8f8f8;
        height: 34px;
        line-height: 34px;
        overflow: hidden;
        display: flex;
        align-items: center;
        padding: 0 10px;

        h3 {
            margin: 0;
            font-size: 14px;
            font-weight: 700;
        }

        em {
            font-style: normal;
            color: #1a1a1a;
            font-weight: 600;
        }

        .st-ext {
            padding-left: 27px;
            font-size: 12px;
            color: #666;

            span {
                color: var(--sort-active-color, #ff4d4f);
                font-weight: 600;
            }
        }
    }

    .filter-top-body {
        background: #fff;
    }

    .list {
        position: relative;
        overflow: hidden;
        padding: 16px 0;
    }

    .attr-name {
        position: absolute;
        top: 9px;
        left: 10px;
        width: 67px;
        font-size: 14px;
        line-height: 23px;
        color: #1a1a1a;
        font-weight: 600;
    }

    .attr-box {
        position: relative;
        display: flex;
        flex-wrap: nowrap;
        align-items: center;
    }

    .category-link-wrapper {
        flex: 1;
        overflow-x: auto;
        overflow-y: hidden;
        scroll-behavior: smooth;

        &::-webkit-scrollbar {
            display: none;
            height: 0;
        }
    }

    .category-link {
        display: flex;
        flex-wrap: nowrap;
        width: 100%;
        min-width: max-content;
    }

    .filter-category__arrow {
        position: absolute;
        top: 0;
        bottom: 0;
        width: 44px;
        display: flex;
        align-items: center;
        justify-content: center;
        cursor: pointer;
        z-index: 2;
        user-select: none;

        &--left {
            left: 0;
            padding-left: 4px;
            padding-right: 12px;
            justify-content: flex-start;
            background: linear-gradient(to right, #fff 60%, rgba(255, 255, 255, 0));
        }

        &--right {
            right: 0;
            padding-left: 12px;
            padding-right: 4px;
            justify-content: flex-end;
            background: linear-gradient(to left, #fff 60%, rgba(255, 255, 255, 0));
        }
    }

    .filter-category__arrow-btn {
        width: 32px;
        height: 32px;
        display: flex;
        align-items: center;
        justify-content: center;
        background: #fff;
        border: 1px solid #ebebeb;
        border-radius: 50%;
        transition: all 0.2s;
        flex-shrink: 0;

        &:hover {
            background: #f7f7f7;
        }
    }

    .filter-category__arrow-icon {
        width: 16px;
        height: 16px;
        display: block;
        color: #1f1f1f;

        &--flip {
            transform: rotate(180deg);
        }
    }

    .item {
        flex: 0 0 auto;
        display: flex;
        align-items: center;
        margin-left: 8px;
        color: #505259;
        white-space: nowrap;
        cursor: pointer;
        text-decoration: none;
        box-sizing: border-box;
        background-color: #fff;
        transition: all 0.2s;
        align-items: center;
        background: #fff;
        border: 1px solid #ebebeb;
        border-radius: 12px;
        display: flex;
        height: 48px;
        max-width: 328px;
        padding: 4px 16px 4px 4px;
        position: relative;
        &:first-child {
            margin-left: 0;
        }

        .category-pic {
            border-radius: 8px;
            height: 40px;
            margin-right: 8px;
            object-fit: contain;
            width: 40px;
        }

        .category-name {
            color: #1f1f1f;
            font-size: 14px;
            font-weight: 500;
            letter-spacing: 0;
            line-height: 22px;
            overflow: hidden;
            text-overflow: ellipsis;
            white-space: nowrap;
        }

        &:hover,
        &.active {
            border-color: var(--general);
            color: #1a1a1a;
            .category-name {
                color: var(--general);
            }
        }

        &.loading {
            pointer-events: none;

            &::after {
                content: "";
                width: 14px;
                height: 14px;
                margin-left: 8px;
                border: 2px solid rgba(255, 77, 79, 0.2);
                border-top-color: var(--sort-active-color, #ff4d4f);
                border-radius: 50%;
                animation: filter-category-spin 0.8s linear infinite;
                flex-shrink: 0;
            }
        }
    }
}

@keyframes filter-category-spin {
    from {
        transform: rotate(0deg);
    }

    to {
        transform: rotate(360deg);
    }
}
</style>
