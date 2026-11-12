<template>
    <div class="filter-container filter-container--top">
        <div class="filter-top">
            <div class="filter-top-body">
                <div v-if="categoryDisplayList.length > 0" class="list">
                    <div class="attr-box">
                        <div v-if="showCategoryLeftArrow" class="filter-category__arrow filter-category__arrow--left" @click="scrollCategory('left')">
                            <span class="filter-category__arrow-btn">
                                <svg class="filter-category__arrow-icon filter-category__arrow-icon--flip" viewBox="0 0 24 24" width="16" height="16">
                                    <path d="M9 6l6 6-6 6" fill="none" stroke="#1f1f1f" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" />
                                </svg>
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
                                    :class="{ active: topCategorySelectedId === String(item.id) }"
                                    @click.capture="handleTopCategoryClick(item.id, $event)"
                                >
                                    <img v-if="item.categoryPic" :src="imageFormat(item.categoryPic)" :alt="item.categoryName" class="category-pic" />
                                    <span class="category-name" :style="[{ paddingLeft: item.categoryPic ? '0' : '12px' }]">{{ item.categoryName }}</span>
                                </NuxtLink>
                            </div>
                        </div>
                        <div v-if="showCategoryRightArrow" class="filter-category__arrow filter-category__arrow--right" @click="scrollCategory('right')">
                            <span class="filter-category__arrow-btn">
                                <svg class="filter-category__arrow-icon" viewBox="0 0 24 24" width="16" height="16">
                                    <path d="M9 6l6 6-6 6" fill="none" stroke="#1f1f1f" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" />
                                </svg>
                            </span>
                        </div>
                    </div>
                </div>
            </div>
        </div>
    </div>
</template>

<script lang="ts" setup>
/**
 * @description 商品搜索页 - 新版顶部分类筛选条
 * - 仅渲染横向分类条；品牌、属性数据拉取后通过 emit 抛给父级供 SortFilter 使用
 * - 数据源：getSearchFilter（分类/品牌）+ getSearchEsFilter（属性）
 */
import { ref, watch, nextTick, onMounted, onBeforeUnmount, type ComponentPublicInstance, type Ref } from "vue";
import { getSearchFilter, getSearchEsFilter } from "@/api/search/search";
import { imageFormat, urlFormat } from "@/utils/format";
import { useSearchStore } from "@/store/search";
import type { filterSeleted, SearchFilter } from "@/types/search/search.d";

type CategoryRow = { id: number | string; categoryName: string; categoryPic?: string; seoLink?: string };
type BrandRow = { id: number | string; brandName: string; brandLogo?: string; firstWord?: string };
type AttrOption = { value: string; checked?: boolean };
type AttrItem = { name: string; values: AttrOption[] };

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
    }
});

const emit = defineEmits(["change", "brandOptionsChange", "panelDataChange"]);

const filter = defineModel<SearchFilter>("filter") as Ref<SearchFilter>;
const filterSeleted = defineModel<filterSeleted>("filterSeleted");
const filterLoading = defineModel<boolean>("filterLoading", { default: false });

const searchStore = useSearchStore();

// ===================== 响应式状态 =====================
const categoryTabsRef = ref<HTMLElement | null>(null);
const categoryLinkRef = ref<HTMLElement | null>(null);
const categoryItemRefs = ref<Record<string, HTMLElement>>({});
// 用 store 中持久化的分类层级做种子，避免路由变化瞬间分类条从空到满的闪烁。
const categoryDisplayList = ref<CategoryRow[]>((searchStore.barCategoryList as CategoryRow[]) || []);
/** 当前选中分类的子分类（二级分类），用于「再次点击同一分类」时就地下钻 */
const catChildren = ref<CategoryRow[]>([]);
const brandList = ref<BrandRow[]>([]);
const attrList = ref<AttrItem[]>([]);
const showCategoryLeftArrow = ref(false);
const showCategoryRightArrow = ref(false);
let categoryResizeObserver: ResizeObserver | null = null;

const topCategorySelectedId = ref(filter.value?.categoryId ? String(filter.value.categoryId) : "");

// ===================== 数据获取 =====================
const resolveCategoryList = (filterData: unknown): CategoryRow[] => {
    const data = (filterData || {}) as Record<string, unknown>;
    const rawList = (Array.isArray(data.category) && data.category) || [];
    return (rawList as Record<string, unknown>[])
        .map((row) => ({
            id: (row.categoryId ?? row.id) as number | string,
            categoryName: String(row.categoryName ?? row.name ?? ""),
            categoryPic: String(row.categoryPic ?? row.pic ?? row.icon ?? ""),
            seoLink: typeof row.seoLink === "string" ? row.seoLink : undefined
        }))
        .filter((item) => item.id !== undefined && item.id !== "" && item.categoryName);
};

const resolveBrandList = (filterData: unknown): BrandRow[] => {
    const data = (filterData || {}) as Record<string, unknown>;
    const rawList = (Array.isArray(data.brand) && data.brand) || [];
    return (rawList as Record<string, unknown>[])
        .map((row) => ({
            id: (row.brandId ?? row.id) as number | string,
            brandName: String(row.brandName ?? row.name ?? ""),
            brandLogo: String(row.brandLogo ?? row.logo ?? ""),
            firstWord: String(row.firstWord ?? row.initial ?? "")
        }))
        .filter((item) => item.id !== undefined && item.id !== "" && item.brandName);
};

const emitPanelDataChange = (): void => {
    emit("panelDataChange", {
        brand: brandList.value,
        category: categoryDisplayList.value,
        attrs: attrList.value
    });
};

const getFilterData = async (): Promise<void> => {
    if (filterLoading.value) return;
    try {
        filterLoading.value = true;
        const result = await getSearchFilter({ ...props.queryParams, page_type: props.pageType });
        brandList.value = resolveBrandList(result.filter);
        const categories = resolveCategoryList(result.filter);
        const selectedCat = filter.value?.categoryId ? String(filter.value.categoryId) : "";
        topCategorySelectedId.value = selectedCat;
        // 接口在选中某分类时返回的是该分类的子分类，先退避，供「再次点击同一分类」时就地下钻使用
        catChildren.value = categories;
        // 分类条层级跨路由保持：仅在「无选中分类 / 切换了搜索上下文 / 无缓存」时，用当前返回重置层级；
        // 选中某分类（第一次点击）时不改层级，保证分类条停留在当前层级，只高亮选中项并筛选商品。
        const baseKey = `${props.pageType}|${filter.value?.keyword || ""}`;
        if (!selectedCat || searchStore.barCategoryKey !== baseKey || searchStore.barCategoryList.length === 0) {
            searchStore.barCategoryList = categories;
            searchStore.barCategoryKey = baseKey;
        }
        categoryDisplayList.value = searchStore.barCategoryList as CategoryRow[];
        filterSeleted.value = (result.filterSelected ?? {}) as filterSeleted;
        emit("brandOptionsChange", brandList.value);
        emitPanelDataChange();
    } catch (error) {
        console.error("[SearchTopFilter] 获取搜索筛选项失败:", error);
    } finally {
        filterLoading.value = false;
    }
};

const getEsFilterData = async (): Promise<void> => {
    try {
        const result = (await getSearchEsFilter({ keyword: props.queryParams.keyword, cat: props.queryParams.cat })) as { attributes?: AttrItem[] };
        const attributes = result.attributes ?? [];
        attributes.forEach((item) => {
            item.values?.forEach((attr) => {
                attr.checked = false;
            });
        });
        attrList.value = attributes;
        emitPanelDataChange();
    } catch (error) {
        console.error("[SearchTopFilter] 获取属性筛选项失败:", error);
    }
};

// ===================== 分类交互 =====================
/** 顶部分类项的可爬取链接；普通点击仍由筛选逻辑处理。 */
const categoryLink = (category: CategoryRow): string => {
    return urlFormat({ path: "list", seoLink: category.seoLink, id: category.id });
};

const handleTopCategoryClick = (categoryId: number | string, event?: MouseEvent): void => {
    // 普通左键继续筛选；修饰键/中键交给 NuxtLink 打开 SEO 链接。
    if (event && (event.button !== 0 || event.metaKey || event.ctrlKey || event.shiftKey || event.altKey)) {
        return;
    }
    event?.preventDefault();
    const nextId = String(categoryId ?? "");
    if (!nextId) return;
    const isSelected = topCategorySelectedId.value === nextId;
    if (isSelected) {
        // 第二次点击已选中分类：就地切换到它的二级分类（子分类），商品仍按该分类筛选，不重新跳转
        if (catChildren.value.length > 0) {
            searchStore.barCategoryList = catChildren.value;
            categoryDisplayList.value = catChildren.value;
            topCategorySelectedId.value = "";
            void nextTick().then(() => {
                updateCategoryArrows();
                if (categoryTabsRef.value) {
                    categoryTabsRef.value.scrollTo({ left: 0, behavior: "smooth" });
                }
            });
        }
        return;
    }
    // 第一次点击：选中该分类（高亮 + 按其筛选商品），分类条层级保持不变。
    topCategorySelectedId.value = nextId;
    filter.value.categoryId = Number(nextId);
    filter.value.page = 1;
    emit("change");
};

// ===================== 滚动箭头 =====================
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
    el.scrollBy({ left: direction === "left" ? -step : step, behavior: "smooth" });
};

const setCategoryItemRef = (el: Element | ComponentPublicInstance | null, categoryId: number | string): void => {
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
    container.scrollTo({ left: Math.min(Math.max(targetLeft, 0), maxScrollLeft), behavior: "smooth" });
};

// ===================== 副作用 =====================
watch(
    () => [props.queryParams, props.pageType],
    () => {
        getFilterData();
    },
    { deep: true, immediate: true }
);

watch(
    () => filter.value?.keyword,
    () => {
        getEsFilterData();
    },
    { immediate: true }
);

watch(
    () => categoryDisplayList.value,
    async () => {
        categoryItemRefs.value = {};
        await nextTick();
        updateCategoryArrows();
        if (topCategorySelectedId.value) {
            void scrollCategoryItemToCenter(topCategorySelectedId.value);
        }
    },
    { deep: true }
);

onMounted(async () => {
    await nextTick();
    updateCategoryArrows();
    if (topCategorySelectedId.value) {
        void scrollCategoryItemToCenter(topCategorySelectedId.value);
    }
    const el = categoryTabsRef.value;
    if (el && typeof ResizeObserver !== "undefined") {
        categoryResizeObserver = new ResizeObserver(() => updateCategoryArrows());
        categoryResizeObserver.observe(el);
    } else if (typeof window !== "undefined") {
        window.addEventListener("resize", updateCategoryArrows);
    }
});

onBeforeUnmount(() => {
    if (categoryResizeObserver) {
        categoryResizeObserver.disconnect();
        categoryResizeObserver = null;
    }
    if (typeof window !== "undefined") {
        window.removeEventListener("resize", updateCategoryArrows);
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
        background: #fff;
    }

    .filter-top-body {
        background: #fff;
    }

    .list {
        position: relative;
        overflow: hidden;
        padding: 16px 0;
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
        text-decoration: none;
        white-space: nowrap;
        cursor: pointer;
        box-sizing: border-box;
        background: #fff;
        border: 1px solid #ebebeb;
        border-radius: 12px;
        height: 48px;
        max-width: 328px;
        padding: 4px 16px 4px 4px;
        position: relative;
        transition: all 0.2s;

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
    }
}
</style>
