<template>
    <div ref="sortFilterWrapRef" class="sort-filter-wrap">
        <div class="sort-filter-left">
            <div class="sort-item" :class="{ active: filter.sortField == '' }" @click="changeSort('')">
                <div class="sort-name">{{ $t("默认排序") }}</div>
            </div>
            <div class="sort-item" :class="{ active: filter.sortField == 'sale' }" @click="changeSort('sale')">
                <div class="sort-name">{{ $t("销量") }}</div>
            </div>
            <div ref="priceSortRef" class="price-sort" @mouseenter="showPriceDropdown" @mouseleave="hidePriceDropdown">
                <div class="sort-item price-sort__trigger" :class="{ active: priceDropdownVisible || filter.sortField == 'price' }">
                    <div class="sort-name">{{ $t("价格") }}</div>
                    <svg class="price-sort__arrow" :class="{ 'price-sort__arrow--open': priceDropdownVisible }" viewBox="0 0 24 24" width="14" height="14">
                        <path d="M6 9l6 6 6-6" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" />
                    </svg>
                </div>
                <div v-if="priceDropdownVisible" class="price-sort__dropdown">
                    <div
                        class="price-sort__option"
                        :class="{ active: filter.sortField === 'price' && filter.sortOrder === 'asc' }"
                        @click.stop="selectPriceSort('asc')"
                    >
                        {{ $t("从低到高") }}
                    </div>
                    <div
                        class="price-sort__option"
                        :class="{ active: filter.sortField === 'price' && filter.sortOrder === 'desc' }"
                        @click.stop="selectPriceSort('desc')"
                    >
                        {{ $t("从高到低") }}
                    </div>
                </div>
            </div>
            <div class="interval-sort" @mouseenter="showIntervalDropdown" @mouseleave="hideIntervalDropdown">
                <div class="sort-item interval-sort__trigger" :class="{ active: intervalDropdownVisible || hasIntervalValue }">
                    <div class="sort-name">{{ $t("区间") }}</div>
                    <svg class="price-sort__arrow" :class="{ 'price-sort__arrow--open': intervalDropdownVisible }" viewBox="0 0 24 24" width="14" height="14">
                        <path d="M6 9l6 6 6-6" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" />
                    </svg>
                </div>
                <div v-if="intervalDropdownVisible" class="interval-sort__dropdown">
                    <div class="interval-sort__input-row">
                        <div class="interval-sort__input-box">
                            <span class="interval-sort__currency">￥</span>
                            <input v-model="minPrice" type="text" :placeholder="$t('最低价')" />
                        </div>
                        <span class="interval-sort__separator">-</span>
                        <div class="interval-sort__input-box">
                            <span class="interval-sort__currency">￥</span>
                            <input v-model="maxPrice" type="text" :placeholder="$t('最高价')" />
                        </div>
                    </div>
                    <div class="interval-sort__preset-list">
                        <div
                            v-for="item in intervalPresetList"
                            :key="item.key"
                            class="interval-sort__preset-item"
                            :class="{ active: activeIntervalPresetKey === item.key }"
                            @click.stop="selectIntervalPreset(item.min, item.max)"
                        >
                            <div class="interval-sort__preset-title">{{ item.label }}</div>
                            <div class="interval-sort__preset-subtitle">{{ item.rate }}的选择</div>
                        </div>
                    </div>
                    <div class="interval-sort__actions">
                        <button type="button" class="interval-sort__btn interval-sort__btn--reset" @click.stop="resetInterval">
                            {{ $t("重置") }}
                        </button>
                        <button type="button" class="interval-sort__btn interval-sort__btn--submit" @click.stop="submitInterval">
                            {{ $t("确定") }}
                        </button>
                    </div>
                </div>
            </div>
        </div>
        <div class="line"></div>
        <div class="sort-filter-center">
            <div v-if="showCenterLeftArrow" class="filter-category__arrow filter-category__arrow--left" @click="scrollCenter('left')">
                <span class="filter-category__arrow-btn">
                    <svg class="filter-category__arrow-icon filter-category__arrow-icon--flip" viewBox="0 0 24 24" width="16" height="16">
                        <path d="M9 6l6 6-6 6" fill="none" stroke="#1f1f1f" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" />
                    </svg>
                </span>
            </div>
            <div ref="centerTabsRef" class="sort-filter-center__scroll" @scroll="updateCenterArrows">
                <div class="sort-filter-center__content">
                    <div class="sort-item" :class="{ active: filter.sortField == 'time' }" @click="changeSort('time')">
                        <div class="sort-name">{{ $t("新品") }}</div>
                    </div>
                    <div
                        v-if="brandOptions.length > 0"
                        ref="brandSortRef"
                        class="brand-sort"
                        @mouseenter="clearBrandCloseTimer"
                        @mouseleave="scheduleCloseBrandDropdown"
                    >
                        <div
                            class="sort-item brand-sort__trigger"
                            :class="{ active: brandDropdownVisible || filter.brandIds.length > 0 }"
                            @click.stop="toggleBrandDropdown"
                        >
                            <div class="sort-name">{{ $t("品牌") }}</div>
                            <svg class="price-sort__arrow" :class="{ 'price-sort__arrow--open': brandDropdownVisible }" viewBox="0 0 24 24" width="14" height="14">
                                <path d="M6 9l6 6 6-6" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" />
                            </svg>
                        </div>
                        <div v-if="brandDropdownVisible" class="brand-sort__dropdown" :style="brandDropdownStyle">
                            <div
                                v-for="item in brandOptions"
                                :key="String(item.id ?? '')"
                                class="brand-sort__item"
                                :class="{ active: selectedBrandIdStrings.includes(String(item.id ?? '')) }"
                                @click.stop="selectBrand(String(item.id ?? ''))"
                            >
                                <div class="brand-sort__name">{{ item.brandName }}</div>
                            </div>
                        </div>
                    </div>
                </div>
            </div>
            <div v-if="showCenterRightArrow" class="filter-category__arrow filter-category__arrow--right" @click="scrollCenter('right')">
                <span class="filter-category__arrow-btn">
                    <svg class="filter-category__arrow-icon" viewBox="0 0 24 24" width="16" height="16">
                        <path d="M9 6l6 6-6 6" fill="none" stroke="#1f1f1f" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" />
                    </svg>
                </span>
            </div>
        </div>
        <div class="sort-filter-right">
            <div
                ref="filterPanelTriggerRef"
                class="sort-item sort-filter__trigger"
                :class="{ active: filterPanelVisible || filterSelectedCount > 0 }"
                @click.stop="toggleFilterPanel"
            >
                <div class="sort-name">
                    <svg class="sort-filter__trigger-icon" viewBox="0 0 24 24" width="18" height="18">
                        <path d="M3 5h18l-7 8.5V20l-4 1v-7.5z" fill="none" stroke="currentColor" stroke-width="2" stroke-linejoin="round" />
                    </svg>
                    <span>{{ $t("筛选") }}</span>
                </div>
                <span v-if="filterSelectedCount > 0" class="sort-filter__badge">{{ filterSelectedCount }}</span>
            </div>
        </div>
    </div>
    <div v-if="filterPanelVisible" class="sort-filter-panel-mask" :style="filterPanelMaskStyle"></div>
    <div v-if="filterPanelVisible" ref="filterPanelRef" class="sort-filter-panel" :style="filterPanelStyle" @click.stop>
        <div class="sort-filter-panel__header">
            <div class="sort-filter-panel__title">{{ $t("全部筛选") }}</div>
            <button type="button" class="sort-filter-panel__close" @click="closeFilterPanel">×</button>
        </div>
        <div v-if="filterPanelSections.length > 0" class="sort-filter-panel__tabs-wrapper">
            <div v-if="showPanelTabsLeftArrow" class="filter-category__arrow filter-category__arrow--left" @click="scrollPanelTabs('left')">
                <span class="filter-category__arrow-btn">
                    <svg class="filter-category__arrow-icon filter-category__arrow-icon--flip" viewBox="0 0 24 24" width="16" height="16">
                        <path d="M9 6l6 6-6 6" fill="none" stroke="#1f1f1f" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" />
                    </svg>
                </span>
            </div>
            <div ref="filterPanelTabsRef" class="sort-filter-panel__tabs" @scroll="updatePanelTabsArrows">
                <button
                    v-for="section in filterPanelSections"
                    :key="section.key"
                    :data-tab-key="section.key"
                    type="button"
                    class="sort-filter-panel__tab"
                    :class="{ active: activePanelTab === section.key }"
                    @click="scrollToFilterSection(section.key)"
                >
                    <span>{{ section.title }}</span>
                </button>
            </div>
            <div v-if="showPanelTabsRightArrow" class="filter-category__arrow filter-category__arrow--right" @click="scrollPanelTabs('right')">
                <span class="filter-category__arrow-btn">
                    <svg class="filter-category__arrow-icon" viewBox="0 0 24 24" width="16" height="16">
                        <path d="M9 6l6 6-6 6" fill="none" stroke="#1f1f1f" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" />
                    </svg>
                </span>
            </div>
        </div>
        <div ref="filterPanelBodyRef" class="sort-filter-panel__body sort-filter-panel__scroll">
            <section
                v-for="section in filterPanelSections"
                :id="`sort-filter-panel-section-${section.key}`"
                :key="section.key"
                class="sort-filter-panel__section"
            >
                <div class="sort-filter-panel__section-title">{{ section.title }}</div>
                <div class="sort-filter-panel__options">
                    <button
                        v-for="option in getVisibleSectionOptions(section)"
                        :key="option.key"
                        type="button"
                        class="sort-filter-panel__option"
                        :class="{ active: option.active }"
                        @click="handleSectionOptionClick(section, option.key, option.label)"
                    >
                        {{ option.label }}
                    </button>
                </div>
                <button
                    v-if="section.options.length > PANEL_SECTION_COLLAPSED_COUNT"
                    type="button"
                    class="sort-filter-panel__toggle"
                    @click="toggleSectionExpand(section.key)"
                >
                    <span>{{ isSectionExpanded(section.key) ? $t("收起") : $t("展开全部") }}</span>
                    <svg
                        class="sort-filter-panel__toggle-arrow"
                        :class="{ 'sort-filter-panel__toggle-arrow--open': isSectionExpanded(section.key) }"
                        viewBox="0 0 24 24"
                        width="14"
                        height="14"
                    >
                        <path d="M6 9l6 6 6-6" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" />
                    </svg>
                </button>
            </section>
            <div v-if="filterPanelSections.length === 0" class="sort-filter-panel__empty">{{ $t("暂无筛选项") }}</div>
        </div>
        <div class="sort-filter-panel__footer">
            <button
                type="button"
                class="sort-filter-panel__reset"
                :disabled="filterSelectedCount === 0"
                :class="{ disabled: filterSelectedCount === 0 }"
                @click="resetFilterPanel"
            >
                {{ $t("重置筛选") }}
            </button>
        </div>
    </div>
</template>

<script lang="ts" setup>
/**
 * @description 商品搜索页 - 新版排序/筛选条
 * - 排序、价格区间、品牌下拉、全部筛选浮层
 * - 属性走独立 attrs 模型（对接 useSearchPage 的 attrList）
 */
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch, watchEffect, type PropType, type Ref } from "vue";
import { useI18n } from "vue-i18n";
import { useSearchStore } from "@/store/search";
import type { SearchFilter } from "@/types/search/search.d";

const { t } = useI18n();
const searchStore = useSearchStore();

type BrandRow = { id: number | string; brandName: string };
type AttrOption = { value: string };
type AttrItem = { name: string; values: AttrOption[] };
type FilterPanelData = {
    brand: BrandRow[];
    category: any[];
    attrs: AttrItem[];
};
type FilterPanelSection = {
    key: string;
    title: string;
    type: "brand" | "attr" | "other";
    selectedCount: number;
    options: Array<{ key: string; label: string; active: boolean }>;
};

const props = defineProps({
    brandOptions: {
        type: Array as PropType<BrandRow[]>,
        default: () => []
    },
    filterPanelData: {
        type: Object as PropType<FilterPanelData>,
        default: () => ({ brand: [], category: [], attrs: [] })
    }
});

const emit = defineEmits(["change", "panelVisibleChange"]);

const filter = defineModel<SearchFilter>("filter") as Ref<SearchFilter>;
const attrs = defineModel<any[]>("attrs", { default: () => [] }) as Ref<any[]>;

// ===================== 响应式状态 =====================
const minPrice = ref("");
const maxPrice = ref("");
const priceDropdownVisible = ref(false);
const priceSortRef = ref<HTMLElement | null>(null);
let showPriceTimer: ReturnType<typeof setTimeout> | null = null;
let hidePriceTimer: ReturnType<typeof setTimeout> | null = null;
const intervalDropdownVisible = ref(false);
let showIntervalTimer: ReturnType<typeof setTimeout> | null = null;
let hideIntervalTimer: ReturnType<typeof setTimeout> | null = null;
const brandDropdownVisible = ref(false);
const brandSortRef = ref<HTMLElement | null>(null);
let hideBrandTimer: ReturnType<typeof setTimeout> | null = null;
const centerTabsRef = ref<HTMLElement | null>(null);
const showCenterLeftArrow = ref(false);
const showCenterRightArrow = ref(false);
const showPanelTabsLeftArrow = ref(false);
const showPanelTabsRightArrow = ref(false);
let centerResizeObserver: ResizeObserver | null = null;
let filterPanelLayoutObserver: ResizeObserver | null = null;
let viewportChangeRafId: number | null = null;
const sortFilterWrapRef = ref<HTMLElement | null>(null);
const filterPanelTriggerRef = ref<HTMLElement | null>(null);
const filterPanelRef = ref<HTMLElement | null>(null);
const filterPanelTabsRef = ref<HTMLElement | null>(null);
const filterPanelBodyRef = ref<HTMLElement | null>(null);
// 用 store 持久化开启态做种子，跨路由变化恢复，保证「点选筛选项时浮层不被关闭」。
const filterPanelVisible = ref(searchStore.filterPanelOpen);
const activePanelTab = ref("");
const filterPanelTop = ref(0);
const filterPanelRight = ref(16);
const filterPanelHeight = ref(0);
const filterPanelMaskTop = ref(0);
const filterPanelMaskHeight = ref(0);
const brandDropdownTop = ref(0);
const brandDropdownLeft = ref(0);
const expandedSectionKeys = ref<string[]>([]);
const PANEL_SECTION_COLLAPSED_COUNT = 6;
const DROPDOWN_SHOW_DELAY = 150;
const DROPDOWN_HIDE_DELAY = 220;
const intervalPresetList = [
    { key: "0-69", min: 0, max: 69, label: "0-69", rate: "30%" },
    { key: "69-308", min: 69, max: 308, label: "69-308", rate: "60%" },
    { key: "308-1299", min: 308, max: 1299, label: "308-1299", rate: "9%" },
    { key: "1299-9999999999", min: 1299, max: 9999999999, label: "1299-9999999999", rate: "1%" }
];

// ===================== 计算属性 =====================
const selectedBrandIdStrings = computed(() => (filter.value.brandIds || []).map((id) => String(id)));

const hasIntervalValue = computed(() => Boolean(filter.value.minPrice || filter.value.maxPrice));

const activeIntervalPresetKey = computed(() => {
    const min = minPrice.value === "" ? "" : Number(minPrice.value);
    const max = maxPrice.value === "" ? "" : Number(maxPrice.value);
    const matched = intervalPresetList.find((item) => item.min === min && item.max === max);
    return matched?.key || "";
});

const getSelectedAttrValues = (attrName: string): string[] => {
    const matched = (attrs.value || []).find((item) => String(item.attrName || "") === attrName);
    if (!matched?.attrValue) return [];
    return Array.isArray(matched.attrValue) ? matched.attrValue.map((item: unknown) => String(item)) : [String(matched.attrValue)];
};

function isAttrValueSelected(attrName: string, attrValue: string): boolean {
    return getSelectedAttrValues(attrName).includes(attrValue);
}

const filterSelectedCount = computed(() => {
    const brandCount = filter.value.brandIds.length;
    const attrCount = (attrs.value || []).reduce((total, item) => {
        const rawValue = item.attrValue;
        if (Array.isArray(rawValue)) {
            return total + rawValue.length;
        }
        return rawValue ? total + 1 : total;
    }, 0);
    const otherCount = filter.value.sortField === "time" ? 1 : 0;
    return brandCount + attrCount + otherCount;
});

const filterPanelSections = computed<FilterPanelSection[]>(() => {
    const sections: FilterPanelSection[] = [];

    if ((props.filterPanelData.brand?.length ?? 0) > 0) {
        sections.push({
            key: "brand",
            title: t("品牌"),
            type: "brand",
            selectedCount: filter.value.brandIds.length,
            options: (props.filterPanelData.brand ?? []).map((item) => ({
                key: String(item.id ?? ""),
                label: String(item.brandName ?? ""),
                active: selectedBrandIdStrings.value.includes(String(item.id ?? ""))
            }))
        });
    }

    (props.filterPanelData.attrs ?? []).forEach((item, index) => {
        sections.push({
            key: `attr-${index}`,
            title: item.name,
            type: "attr",
            selectedCount: getSelectedAttrValues(item.name).length,
            options: item.values.map((option) => ({
                key: option.value,
                label: option.value,
                active: isAttrValueSelected(item.name, option.value)
            }))
        });
    });

    sections.push({
        key: "other",
        title: t("其他"),
        type: "other",
        selectedCount: filter.value.sortField === "time" ? 1 : 0,
        options: [{ key: "time", label: t("新品"), active: filter.value.sortField === "time" }]
    });

    return sections;
});

const filterPanelStyle = computed(() => {
    const panelTop = Math.max(filterPanelTop.value, 0);
    const h = `${Math.max(filterPanelHeight.value, 0)}px`;
    return { top: `${panelTop}px`, right: `${filterPanelRight.value}px`, height: h, maxHeight: h };
});

const filterPanelMaskStyle = computed(() => ({
    top: `${Math.max(filterPanelMaskTop.value, 0)}px`,
    height: `${Math.max(filterPanelMaskHeight.value, 0)}px`
}));

const brandDropdownStyle = computed(() => ({ top: `${brandDropdownTop.value}px`, left: `${brandDropdownLeft.value}px` }));

// ===================== 排序 =====================
const changeSort = (sort: string): void => {
    if (sort === "") {
        filter.value.sortField = "";
        filter.value.sortOrder = "";
    } else if (sort === "sale") {
        filter.value.sortField = "sale";
        filter.value.sortOrder = "";
    } else if (sort === "time") {
        if (filter.value.sortField === "time") {
            filter.value.sortField = "";
            filter.value.sortOrder = "";
        } else {
            filter.value.sortField = "time";
            filter.value.sortOrder = "";
        }
    } else if (sort === "price") {
        filter.value.sortField = "price";
        filter.value.sortOrder = filter.value.sortOrder == "asc" ? "desc" : "asc";
    }
    filter.value.page = 1;
    emit("change");
};

const selectPriceSort = (order: "asc" | "desc"): void => {
    filter.value.sortField = "price";
    filter.value.sortOrder = order;
    filter.value.page = 1;
    priceDropdownVisible.value = false;
    emit("change");
};

// ===================== 价格区间 =====================
const selectIntervalPreset = (min: number, max: number): void => {
    minPrice.value = String(min);
    maxPrice.value = String(max);
};

const resetInterval = (): void => {
    minPrice.value = "";
    maxPrice.value = "";
};

const submitInterval = (): void => {
    filter.value.minPrice = minPrice.value ? Number(minPrice.value) : null;
    filter.value.maxPrice = maxPrice.value ? Number(maxPrice.value) : null;
    filter.value.page = 1;
    intervalDropdownVisible.value = false;
    emit("change");
};

// ===================== 品牌 =====================
const selectBrand = (brandId: string): void => {
    const id = Number(brandId);
    if (filter.value.brandIds.map((item) => String(item)).includes(brandId)) {
        filter.value.brandIds = filter.value.brandIds.filter((item) => String(item) !== brandId);
    } else {
        filter.value.brandIds = [...filter.value.brandIds, id];
    }
    filter.value.page = 1;
    emit("change");
};

// ===================== 属性 =====================
const toggleAttrValue = (sectionKey: string, attrValue: string): void => {
    const section = filterPanelSections.value.find((item) => item.key === sectionKey);
    if (!section || section.type !== "attr") return;

    const attrName = section.title;
    const nextAttrs = [...(attrs.value || [])] as Array<{ attrName?: string; attrValue?: string[] | string }>;
    const targetIndex = nextAttrs.findIndex((item) => String(item.attrName || "") === attrName);

    if (targetIndex === -1) {
        nextAttrs.push({ attrName, attrValue: [attrValue] });
    } else {
        const currentAttr = nextAttrs[targetIndex];
        const currentValues = Array.isArray(currentAttr?.attrValue)
            ? [...(currentAttr.attrValue as string[])]
            : currentAttr?.attrValue
              ? [String(currentAttr.attrValue)]
              : [];
        const valueIndex = currentValues.indexOf(attrValue);
        if (valueIndex > -1) {
            currentValues.splice(valueIndex, 1);
        } else {
            currentValues.push(attrValue);
        }
        if (currentValues.length === 0) {
            nextAttrs.splice(targetIndex, 1);
        } else {
            nextAttrs[targetIndex] = { attrName, attrValue: currentValues };
        }
    }

    attrs.value = nextAttrs;
    filter.value.page = 1;
    emit("change");
};

const handleSectionOptionClick = (section: FilterPanelSection, optionKey: string, optionLabel: string): void => {
    if (section.type === "brand") {
        selectBrand(optionKey);
        return;
    }
    if (section.type === "other") {
        if (optionKey === "time") {
            changeSort("time");
        }
        return;
    }
    toggleAttrValue(section.key, optionLabel);
};

const resetFilterPanel = (): void => {
    if (filterSelectedCount.value === 0) return;
    filter.value.brandIds = [];
    attrs.value = [];
    filter.value.minPrice = null;
    filter.value.maxPrice = null;
    filter.value.sortField = "";
    minPrice.value = "";
    maxPrice.value = "";
    filter.value.page = 1;
    emit("change");
};

// ===================== 下拉显示/隐藏 =====================
const showPriceDropdown = (): void => {
    if (showPriceTimer) clearTimeout(showPriceTimer);
    if (hidePriceTimer) {
        clearTimeout(hidePriceTimer);
        hidePriceTimer = null;
    }
    showPriceTimer = setTimeout(() => {
        priceDropdownVisible.value = true;
        showPriceTimer = null;
    }, DROPDOWN_SHOW_DELAY);
};

const hidePriceDropdown = (): void => {
    if (showPriceTimer) {
        clearTimeout(showPriceTimer);
        showPriceTimer = null;
    }
    if (hidePriceTimer) clearTimeout(hidePriceTimer);
    hidePriceTimer = setTimeout(() => {
        priceDropdownVisible.value = false;
        hidePriceTimer = null;
    }, DROPDOWN_HIDE_DELAY);
};

const showIntervalDropdown = (): void => {
    if (showIntervalTimer) clearTimeout(showIntervalTimer);
    if (hideIntervalTimer) {
        clearTimeout(hideIntervalTimer);
        hideIntervalTimer = null;
    }
    showIntervalTimer = setTimeout(() => {
        intervalDropdownVisible.value = true;
        showIntervalTimer = null;
    }, DROPDOWN_SHOW_DELAY);
};

const hideIntervalDropdown = (): void => {
    if (showIntervalTimer) {
        clearTimeout(showIntervalTimer);
        showIntervalTimer = null;
    }
    if (hideIntervalTimer) clearTimeout(hideIntervalTimer);
    hideIntervalTimer = setTimeout(() => {
        intervalDropdownVisible.value = false;
        hideIntervalTimer = null;
    }, DROPDOWN_HIDE_DELAY);
};

const syncBrandDropdownPosition = (): void => {
    if (!brandSortRef.value || typeof window === "undefined") return;
    const rect = brandSortRef.value.getBoundingClientRect();
    const dropdownWidth = 400;
    const safePadding = 16;
    const centeredLeft = rect.left + rect.width / 2 - dropdownWidth / 2;
    const maxLeft = Math.max(window.innerWidth - dropdownWidth - safePadding, safePadding);
    brandDropdownTop.value = rect.bottom + 4;
    brandDropdownLeft.value = Math.min(Math.max(centeredLeft, safePadding), maxLeft);
};

const toggleBrandDropdown = async (): Promise<void> => {
    clearBrandCloseTimer();
    filterPanelVisible.value = false;
    brandDropdownVisible.value = !brandDropdownVisible.value;
    if (brandDropdownVisible.value) {
        await nextTick();
        syncBrandDropdownPosition();
    }
};

const closeBrandDropdown = (): void => {
    brandDropdownVisible.value = false;
};

const clearBrandCloseTimer = (): void => {
    if (hideBrandTimer) {
        clearTimeout(hideBrandTimer);
        hideBrandTimer = null;
    }
};

const scheduleCloseBrandDropdown = (): void => {
    clearBrandCloseTimer();
    hideBrandTimer = setTimeout(() => {
        closeBrandDropdown();
        hideBrandTimer = null;
    }, DROPDOWN_HIDE_DELAY);
};

const closeFloatingPanels = (): void => {
    if (showPriceTimer) {
        clearTimeout(showPriceTimer);
        showPriceTimer = null;
    }
    if (hidePriceTimer) {
        clearTimeout(hidePriceTimer);
        hidePriceTimer = null;
    }
    if (showIntervalTimer) {
        clearTimeout(showIntervalTimer);
        showIntervalTimer = null;
    }
    if (hideIntervalTimer) {
        clearTimeout(hideIntervalTimer);
        hideIntervalTimer = null;
    }
    clearBrandCloseTimer();
    priceDropdownVisible.value = false;
    intervalDropdownVisible.value = false;
    brandDropdownVisible.value = false;
};

// ===================== 筛选浮层定位 =====================
const syncFilterPanelPosition = (): void => {
    if (!sortFilterWrapRef.value || typeof window === "undefined") return;
    const rect = sortFilterWrapRef.value.getBoundingClientRect();
    const rightWrapRect = sortFilterWrapRef.value.closest(".right-wrap")?.getBoundingClientRect();
    const pageBodyRect = sortFilterWrapRef.value.closest(".page-body")?.getBoundingClientRect();
    const visibleTop = Math.max(pageBodyRect?.top ?? 0, 0);
    const visibleBottom = Math.min(rightWrapRect?.bottom ?? window.innerHeight, window.innerHeight);
    filterPanelTop.value = rect.bottom;
    filterPanelRight.value = Math.max(window.innerWidth - rect.right - 14, 16);
    filterPanelHeight.value = Math.max(visibleBottom - filterPanelTop.value, 0);
    filterPanelMaskTop.value = visibleTop;
    filterPanelMaskHeight.value = Math.max(visibleBottom - visibleTop, 0);
};

const disconnectFilterPanelLayoutObserver = (): void => {
    if (!filterPanelLayoutObserver) return;
    filterPanelLayoutObserver.disconnect();
    filterPanelLayoutObserver = null;
};

const observeFilterPanelLayout = (): void => {
    disconnectFilterPanelLayoutObserver();
    if (typeof ResizeObserver === "undefined" || !sortFilterWrapRef.value) return;
    const targets = [
        sortFilterWrapRef.value,
        sortFilterWrapRef.value.closest(".right-wrap"),
        sortFilterWrapRef.value.closest(".page-body"),
        filterPanelRef.value
    ].filter((item): item is Element => Boolean(item));
    if (targets.length === 0) return;
    filterPanelLayoutObserver = new ResizeObserver(() => {
        if (!filterPanelVisible.value) return;
        syncFilterPanelPosition();
        updatePanelTabsArrows();
    });
    targets.forEach((target) => filterPanelLayoutObserver?.observe(target));
};

const closeFilterPanel = (): void => {
    filterPanelVisible.value = false;
};

const toggleFilterPanel = async (): Promise<void> => {
    if (filterPanelVisible.value) {
        closeFilterPanel();
        return;
    }
    closeFloatingPanels();
    clearBrandCloseTimer();
    filterPanelVisible.value = true;
    activePanelTab.value = filterPanelSections.value[0]?.key || "";
    await nextTick();
    syncFilterPanelPosition();
};

// 供父级在「顶部分类条切换分类」时同步收起浮层。
defineExpose({ closeFilterPanel });

// ===================== 滚动箭头 =====================
const updateCenterArrows = (): void => {
    const el = centerTabsRef.value;
    if (!el) {
        showCenterLeftArrow.value = false;
        showCenterRightArrow.value = false;
        return;
    }
    showCenterLeftArrow.value = el.scrollLeft > 0;
    showCenterRightArrow.value = el.scrollLeft + el.clientWidth < el.scrollWidth - 1;
};

const scrollCenter = (direction: "left" | "right"): void => {
    const el = centerTabsRef.value;
    if (!el) return;
    const step = el.clientWidth * 0.6;
    el.scrollBy({ left: direction === "left" ? -step : step, behavior: "smooth" });
};

const updatePanelTabsArrows = (): void => {
    const el = filterPanelTabsRef.value;
    if (!el) {
        showPanelTabsLeftArrow.value = false;
        showPanelTabsRightArrow.value = false;
        return;
    }
    showPanelTabsLeftArrow.value = el.scrollLeft > 0;
    showPanelTabsRightArrow.value = el.scrollLeft + el.clientWidth < el.scrollWidth - 1;
};

const scrollPanelTabs = (direction: "left" | "right"): void => {
    const el = filterPanelTabsRef.value;
    if (!el) return;
    const step = el.clientWidth * 0.6;
    el.scrollBy({ left: direction === "left" ? -step : step, behavior: "smooth" });
};

// ===================== 浮层分区滚动 =====================
const scrollToFilterSection = (sectionKey: string): void => {
    activePanelTab.value = sectionKey;
    const wrapEl = filterPanelBodyRef.value;
    if (!wrapEl) return;
    const sectionEl = wrapEl.querySelector(`#sort-filter-panel-section-${sectionKey}`) as HTMLElement | null;
    if (sectionEl) {
        wrapEl.scrollTop = sectionEl.offsetTop;
    }
    scrollTabToCenter(sectionKey);
};

const scrollTabToCenter = (sectionKey: string): void => {
    const tabsContainer = filterPanelTabsRef.value;
    if (!tabsContainer) return;
    const tabEl = tabsContainer.querySelector(`[data-tab-key="${sectionKey}"]`) as HTMLElement | null;
    if (!tabEl) return;
    const containerWidth = tabsContainer.clientWidth;
    const tabLeft = tabEl.offsetLeft;
    const tabWidth = tabEl.offsetWidth;
    tabsContainer.scrollTo({ left: tabLeft - containerWidth / 2 + tabWidth / 2, behavior: "smooth" });
};

const isSectionExpanded = (sectionKey: string): boolean => expandedSectionKeys.value.includes(sectionKey);

const toggleSectionExpand = (sectionKey: string): void => {
    if (isSectionExpanded(sectionKey)) {
        expandedSectionKeys.value = expandedSectionKeys.value.filter((item) => item !== sectionKey);
        return;
    }
    expandedSectionKeys.value = [...expandedSectionKeys.value, sectionKey];
};

const getVisibleSectionOptions = (section: FilterPanelSection): FilterPanelSection["options"] => {
    if (isSectionExpanded(section.key)) {
        return section.options;
    }
    return section.options.slice(0, PANEL_SECTION_COLLAPSED_COUNT);
};

// ===================== 文档点击/视口监听 =====================
const handleDocumentClick = (event: MouseEvent): void => {
    const target = event.target as Node | null;
    if (brandDropdownVisible.value && !(target && brandSortRef.value?.contains(target))) {
        closeBrandDropdown();
    }
};

const handleViewportChange = (): void => {
    if (viewportChangeRafId !== null) return;
    viewportChangeRafId = requestAnimationFrame(() => {
        viewportChangeRafId = null;
        if (brandDropdownVisible.value) {
            syncBrandDropdownPosition();
        }
        if (!filterPanelVisible.value) return;
        syncFilterPanelPosition();
    });
};

// ===================== 副作用 =====================
watchEffect(() => {
    minPrice.value = filter.value.minPrice ? String(filter.value.minPrice) : "";
    maxPrice.value = filter.value.maxPrice ? String(filter.value.maxPrice) : "";
});

watch(
    () => props.brandOptions.length,
    async () => {
        await nextTick();
        updateCenterArrows();
    },
    { immediate: true }
);

watch(
    () => filterPanelSections.value,
    (sections) => {
        if (!sections.some((item) => item.key === activePanelTab.value)) {
            activePanelTab.value = sections[0]?.key || "";
        }
        expandedSectionKeys.value = expandedSectionKeys.value.filter((key) => sections.some((item) => item.key === key));
        if (filterPanelVisible.value) {
            nextTick(() => {
                syncFilterPanelPosition();
                observeFilterPanelLayout();
                updatePanelTabsArrows();
            });
        }
    },
    { deep: true, immediate: true }
);

watch(
    () => filterPanelVisible.value,
    (visible) => {
        searchStore.filterPanelOpen = visible;
        emit("panelVisibleChange", visible);
        if (visible) {
            nextTick(() => {
                syncFilterPanelPosition();
                observeFilterPanelLayout();
                updatePanelTabsArrows();
            });
            return;
        }
        disconnectFilterPanelLayoutObserver();
    },
    { immediate: true }
);

if (typeof document !== "undefined") {
    document.addEventListener("click", handleDocumentClick);
}

onMounted(async () => {
    await nextTick();
    updateCenterArrows();
    observeFilterPanelLayout();
    const el = centerTabsRef.value;
    if (el && typeof ResizeObserver !== "undefined") {
        centerResizeObserver = new ResizeObserver(() => updateCenterArrows());
        centerResizeObserver.observe(el);
    } else if (typeof window !== "undefined") {
        window.addEventListener("resize", updateCenterArrows);
    }
    if (typeof window !== "undefined") {
        window.addEventListener("resize", handleViewportChange);
        window.addEventListener("scroll", handleViewportChange, true);
    }
});

onBeforeUnmount(() => {
    if (showPriceTimer) clearTimeout(showPriceTimer);
    if (hidePriceTimer) clearTimeout(hidePriceTimer);
    if (showIntervalTimer) clearTimeout(showIntervalTimer);
    if (hideIntervalTimer) clearTimeout(hideIntervalTimer);
    if (hideBrandTimer) clearTimeout(hideBrandTimer);
    if (centerResizeObserver) {
        centerResizeObserver.disconnect();
        centerResizeObserver = null;
    }
    disconnectFilterPanelLayoutObserver();
    if (viewportChangeRafId !== null) {
        cancelAnimationFrame(viewportChangeRafId);
        viewportChangeRafId = null;
    }
    if (typeof document !== "undefined") {
        document.removeEventListener("click", handleDocumentClick);
    }
    if (typeof window !== "undefined") {
        window.removeEventListener("resize", updateCenterArrows);
        window.removeEventListener("resize", handleViewportChange);
        window.removeEventListener("scroll", handleViewportChange, true);
    }
});
</script>

<style lang="less" scoped>
.sort-filter-wrap {
    box-sizing: border-box;
    display: inline-flex;
    align-items: center;
    max-width: 100%;
    padding: 12px 0 16px 0;
    width: 100%;
    position: sticky;
    top: 0;
    z-index: 20;
    background-color: #fff;

    .line {
        width: 1px;
        height: 16px;
        background-color: #ebebeb;
        margin: 0 12px;
    }

    .sort-item {
        cursor: pointer;
        display: flex;
        align-items: center;
        justify-content: center;
        color: rgb(17, 25, 45);
        padding: 0 12px;
        background-color: #fff;
        transition: all 0.2s;
        border: 1px solid #ebebeb;
        border-radius: 8px;
        font-size: 14px !important;
        height: 32px;
        box-sizing: border-box;

        &.active {
            color: var(--general);
            border-color: var(--general);
        }

        &:hover {
            color: var(--general);
            border-color: var(--general);
        }
    }
}

.sort-filter-left {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 10px;
    min-width: 0;
}

.sort-filter-center,
.sort-filter-right {
    display: flex;
    align-items: center;
    margin-right: 12px;
    flex-shrink: 0;
    .sort-name {
        display: flex;
        align-items: center;
        gap: 2px;
    }
}

.sort-filter-center {
    position: relative;
    min-width: 0;
    flex: 1;
    max-width: 320px;
}

.price-sort,
.interval-sort,
.brand-sort {
    position: relative;
}

.price-sort__trigger,
.interval-sort__trigger,
.brand-sort__trigger {
    justify-content: space-between !important;
}

.price-sort__arrow {
    flex-shrink: 0;
    color: currentColor;
    transform: rotate(0deg);
    transition: transform 0.2s ease;
}

.price-sort__arrow--open {
    transform: rotate(180deg);
}

.price-sort__dropdown {
    position: absolute;
    top: 100%;
    left: 0;
    margin-top: 4px;
    background: #fff;
    z-index: 30;
    border: 1px solid rgba(0, 17, 51, 0.05);
    border-radius: 8px;
    box-shadow: 0 0 6px 0 rgba(0, 17, 51, 0.1);
    padding: 4px;
}

.interval-sort__dropdown {
    position: absolute;
    top: calc(100% + 4px);
    left: 50%;
    transform: translateX(-50%);
    width: 240px;
    z-index: 30;
    box-sizing: border-box;
    background: #fff;
    border-radius: 12px;
    box-shadow:
        0 0 16px 0 rgba(10, 10, 51, 0.08),
        0 0 8px 0 rgba(0, 0, 0, 0.04);
    padding: 12px;
}

.brand-sort__dropdown {
    position: fixed;
    width: 400px;
    max-height: 184px;
    overflow-y: auto;
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 4px;
    background: #fff;
    border-radius: 16px;
    box-shadow: 0 8px 16px 0 rgba(0, 0, 0, 0.2);
    border: 1px solid rgba(0, 0, 0, 0.02);
    z-index: 35;
    box-sizing: border-box;
    padding: 8px;
}

.brand-sort__item {
    min-width: 0;
    cursor: pointer;
    transition: color 0.2s ease;
    height: 48px;
    border-radius: 8px;
    padding: 5px 8px;
    overflow: hidden;
    display: flex;
    flex-direction: column;
    justify-content: center;
    align-items: center;
    box-sizing: border-box;

    &:hover {
        background: rgba(0, 0, 0, 0.04);
    }
    &.active {
        background: color-mix(in srgb, var(--general) 10%, #fff);
        .brand-sort__name {
            color: var(--general);
        }
    }
}

.brand-sort__name {
    overflow: hidden;
    color: rgba(0, 0, 0, 0.92);
    font-size: 14px;
    line-height: 22px;
    text-overflow: ellipsis;
    white-space: nowrap;
    transition: color 0.2s ease;
}

.interval-sort__input-row {
    display: flex;
    align-items: center;
    gap: 12px;
    margin-bottom: 12px;
}

.interval-sort__input-box {
    flex: 1;
    display: flex;
    align-items: center;
    padding: 5px 12px;
    border: 1px solid #ebebeb;
    border-radius: 8px;
    background: #fff;
    transition: border-color 0.2s ease;
    height: 32px;
    justify-content: center;
    box-sizing: border-box;
    width: 92px;

    &:focus-within {
        border-color: var(--general);
    }

    input {
        flex: 1;
        min-width: 0;
        border: none;
        outline: none;
        font-size: 14px;
        line-height: 22px;
        color: #1f1f1f;
        background: transparent;
    }
}

.interval-sort__currency,
.interval-sort__separator {
    color: #9aa3b2;
    font-size: 14px;
}

.interval-sort__separator {
    flex-shrink: 0;
}

.interval-sort__preset-list {
    display: flex;
    flex-direction: column;
    gap: 8px;
    margin-bottom: 8px;
}

.interval-sort__preset-item {
    border: 1px solid #ebebeb;
    border-radius: 8px;
    height: 48px;
    display: flex;
    flex-direction: column;
    justify-content: center;
    align-items: center;
    cursor: pointer;
    transition:
        border-color 0.2s ease,
        background-color 0.2s ease,
        color 0.2s ease;
    &.active {
        border-color: var(--general);
        color: var(--general);
        .interval-sort__preset-title,
        .interval-sort__preset-subtitle {
            color: var(--general);
        }
    }
}

.interval-sort__preset-title {
    font-size: 14px;
    line-height: 22px;
    color: #1f1f1f;
}

.interval-sort__preset-subtitle {
    font-size: 12px;
    line-height: 16px;
    color: #7a7a7a;
}

.interval-sort__actions {
    display: flex;
    align-items: center;
    gap: 12px;
}

.interval-sort__btn {
    border-radius: 8px;
    font-size: 16px;
    font-weight: 600;
    border: 1px solid transparent;
    cursor: pointer;
    transition: all 0.2s ease;
    padding: 8px 16px;
}

.interval-sort__btn--reset {
    color: #1f1f1f;
    background: #fff;
    border-color: #e7eaf0;
}

.interval-sort__btn--submit {
    flex: 1;
    color: #fff;
    background: var(--general);
}

.price-sort__option {
    padding: 8px 16px;
    white-space: nowrap;
    font-size: 14px;
    line-height: 24px;
    color: rgb(17, 25, 45);
    cursor: pointer;
    transition:
        color 0.2s ease,
        background-color 0.2s ease;

    &:hover,
    &.active {
        color: var(--general);
        background: #f1f1f1;
        border-radius: 4px;
    }
    &.active {
        font-weight: 600;
    }
}

.sort-filter-right {
    margin-left: auto;
}

.sort-filter__trigger {
    position: relative;
    .sort-name {
        color: inherit;
    }
}

.sort-filter__trigger-icon {
    color: currentColor;
    transition: color 0.2s ease;
    transform: translateY(1px);
}

.sort-filter__badge {
    position: absolute;
    top: -8px;
    right: -8px;
    min-width: 18px;
    height: 18px;
    padding: 0 5px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border-radius: 999px;
    background: var(--general);
    color: #fff;
    font-size: 12px;
    line-height: 18px;
    font-weight: 600;
    box-sizing: border-box;
}

.sort-filter-center__scroll {
    width: 100%;
    overflow-x: auto;
    overflow-y: visible;
    scroll-behavior: smooth;
    scrollbar-width: none;

    &::-webkit-scrollbar {
        display: none;
        height: 0;
    }
}

.sort-filter-center__content {
    display: inline-flex;
    align-items: center;
    gap: 10px;
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

.sort-filter-panel-mask {
    position: fixed;
    left: 0;
    right: 0;
    z-index: 39;
    pointer-events: none;
}

.sort-filter-panel {
    position: fixed;
    width: 360px;
    max-width: calc(100vw - 32px);
    background: #fff;
    border: 1px solid rgba(0, 0, 0, 0.08);
    border-radius: 16px;
    z-index: 40;
    display: flex;
    flex-direction: column;
    overflow: hidden;
}

.sort-filter-panel__header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 12px 0 12px;
    flex-shrink: 0;
}

.sort-filter-panel__title {
    font-size: 16px;
    font-weight: 600;
    color: rgba(0, 0, 0, 0.92);
    line-height: 24px;
}

.sort-filter-panel__close {
    border: none;
    padding: 10px;
    border-radius: 50%;
    background: transparent;
    color: #1f1f1f;
    font-size: 28px;
    line-height: 36px;
    cursor: pointer;
    transition: background-color 0.2s ease;
}

.sort-filter-panel__tabs-wrapper {
    position: relative;
    display: flex;
    align-items: center;
    flex-shrink: 0;
}

.sort-filter-panel__tabs {
    display: flex;
    align-items: center;
    gap: 24px;
    padding: 0 12px 4px;
    overflow-x: auto;
    scrollbar-width: none;
    flex: 1;
    min-width: 0;

    &::-webkit-scrollbar {
        display: none;
    }
}

.sort-filter-panel__tab {
    position: relative;
    display: inline-flex;
    align-items: center;
    gap: 6px;
    flex: 0 0 auto;
    border: none;
    background: transparent;
    padding: 8px 0 12px;
    color: #333;
    font-size: 16px;
    font-weight: 500;
    cursor: pointer;
    transition: color 0.2s ease;

    &.active {
        color: var(--general);
        font-weight: 600;

        &::after {
            content: "";
            position: absolute;
            left: 0;
            right: 0;
            bottom: 0;
            height: 3px;
            border-radius: 999px;
            background: var(--general);
        }
    }
}

.sort-filter-panel__scroll {
    flex: 1;
    overflow-y: auto;
    padding: 8px 12px 12px;
}

.sort-filter-panel__section {
    & + & {
        margin-top: 24px;
    }
}

.sort-filter-panel__section-title {
    margin-bottom: 8px;
    color: #1f1f1f;
    font-size: 16px;
    font-weight: 600;
    line-height: 24px;
}

.sort-filter-panel__options {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 12px;
}

.sort-filter-panel__toggle {
    margin-top: 12px;
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 0;
    border: none;
    background: transparent;
    color: #666;
    font-size: 14px;
    line-height: 20px;
    cursor: pointer;
    transition: color 0.2s ease;

    &:hover {
        color: var(--general);
    }
}

.sort-filter-panel__toggle-arrow {
    flex-shrink: 0;
    color: currentColor;
    transform: rotate(0deg);
    transition: transform 0.2s ease;
}

.sort-filter-panel__toggle-arrow--open {
    transform: rotate(180deg);
}

.sort-filter-panel__option {
    min-width: 0;
    min-height: 48px;
    padding: 0 12px;
    border: 1px solid #ebebeb;
    border-radius: 12px;
    background: #fff;
    color: rgba(0, 0, 0, 0.92);
    font-size: 14px;
    line-height: 20px;
    cursor: pointer;
    transition:
        border-color 0.2s ease,
        color 0.2s ease,
        background-color 0.2s ease;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;

    &:hover,
    &.active {
        border-color: var(--general);
        color: var(--general);
    }
}

.sort-filter-panel__empty {
    padding: 60px 0;
    text-align: center;
    color: #999;
    font-size: 14px;
}

.sort-filter-panel__footer {
    flex-shrink: 0;
    padding: 12px;
    background: linear-gradient(to top, #fff 75%, rgba(255, 255, 255, 0.92));
    .disabled {
        opacity: 0.36;
        cursor: not-allowed;
    }
}

.sort-filter-panel__reset {
    width: 100%;
    height: 40px;
    border: none;
    border-radius: 8px;
    background: var(--general);
    color: #fff;
    font-size: 16px;
    font-weight: 600;
    cursor: pointer;
}
</style>
