<template>
    <div style="background-color: #fff">
        <CommonPageHeader></CommonPageHeader>
        <CommonHeader :title="filter.keyword ? `${filter.keyword} - 商品搜索` : '商品搜索'"></CommonHeader>
        <div class="container">
            <!--            <shopInfo :keyword="filter.keyword" v-if="filter.keyword" style="flex-shrink: 0"></shopInfo>-->
            <CommonCategoryNavigation v-model:id="filter.categoryId" isSearchPage @change="onFilterChange" style="flex-shrink: 0">
                <template v-if="filterSeleted">
                    <div class="nav-selected" v-if="filter.brandIds.length > 0 && filterSeleted.brand">
                        <div
                            class="item"
                            @click="
                                filter.brandIds = [];
                                onFilterChange();
                            "
                        >
                            <span class="item-label">{{ $t("品牌") }}：</span><span class="item-value">{{ filterSeleted.brand }}</span>
                        </div>
                    </div>
                    <div class="nav-selected" v-if="hasPriceFilter">
                        <div
                            class="item"
                            @click="
                                filter.maxPrice = null;
                                filter.minPrice = null;
                                onFilterChange();
                            "
                        >
                            <span class="item-label">{{ $t("价格") }}：</span
                            ><span class="item-value">{{ selectedPriceText }}</span>
                        </div>
                    </div>
                    <template v-if="attrList.length > 0 && filterSeleted.attrs">
                        <div class="nav-selected">
                            <div v-for="(value, key) in filterSeleted.attrs" :key="key" class="item" @click="clearAttr(value.attrName)">
                                <span class="item-label">{{ value.attrName }}：</span>
                                <span class="item-value">{{ formatSelectedAttrValue(value.attrValue) }}</span>
                            </div>
                        </div>
                    </template>
                </template>

                <div class="nav-selected" v-if="filter.keyword">
                    <div
                        class="item"
                        v-if="filter.keyword"
                        @click="
                            filter.keyword = '';
                            onFilterChange();
                        "
                    >
                        <span class="item-label">{{ $t("搜索") }}：</span><span class="item-value">"{{ filter.keyword }}"</span>
                    </div>
                </div>
            </CommonCategoryNavigation>
            <div ref="FilterRef">
                <CommonFilter
                    ref="filterRef"
                    v-model:filter="filter"
                    v-model:filterSeleted="filterSeleted"
                    v-model:queryParams="queryParams"
                    :pageType="pageType"
                    :attrList="attrList"
                    isSearchPage
                    @change="onFilterChange"
                    @changeAttr="onFilterAttrChange"
                ></CommonFilter>
            </div>
            <CommonCatGoodsList
                v-model:queryParams="queryParams"
                v-model:filter="filter"
                :attrs="attrList"
                @change="onFilterChange"
                @pageChange="onPageChange"
                @filter-undo="onFilterUndo"
            />
        </div>
        <CommonPageFooter></CommonPageFooter>
    </div>
</template>
<script lang="ts" setup>
/**
 * @description 商品搜索/列表页 - 经典版 UI 布局（左侧分类导航 + 侧栏筛选 + 商品列表）
 * - 仅负责展示，业务逻辑统一来自 useSearchPage 上下文（唯一真理源）
 * - 支持关键词搜索、分类筛选、品牌筛选、价格区间筛选、属性筛选、多种排序
 */
import { ref, computed, watch, type Ref } from "vue";
import { useI18n } from "vue-i18n";
import { useSearchPageContext } from "@/composables/useSearchPage";
// import shopInfo from "./shopInfo.vue";

const { pageType, filter, filterSeleted, attrList, queryParams, onFilterChange, onFilterAttrChange, onFilterUndo, clearAttr } = useSearchPageContext();

const { t } = useI18n();
const filterRef = ref();

const hasPriceFilter = computed(() => (filter.minPrice ?? 0) > 0 || (filter.maxPrice ?? 0) > 0);
const selectedPriceText = computed(() => {
    const min = filter.minPrice ?? "";
    const max = filter.maxPrice ?? null;
    return max != null && max > 0 ? `${min} - ${max}` : `${min}${t("以上")}`;
});

const formatSelectedAttrValue = (values: string[] = []) => {
    return values.join("，");
};

watch(
    () => filterSeleted.value,
    (newVal) => {
        if (newVal?.brand == null) {
            filterRef.value?.clearMultipleBrandIds();
        }
    }
);

const FilterRef: Ref<HTMLElement | null> = ref(null);
/** 分页变更时滚动到筛选区域顶部，提升用户体验 */
const onPageChange = (): void => {
    const targetElement = FilterRef.value as HTMLElement;
    if (targetElement) {
        targetElement.scrollIntoView();
    }
};
</script>
<style lang="less" scoped>
.container {
    .nav-selected {
        min-width: 10px;
        display: flex;
        flex-wrap: wrap;
        .item {
            display: inline-flex;
            align-items: center;
            min-width: 0;
            max-width: 280px;
            box-sizing: border-box;
            overflow: hidden;
            white-space: nowrap;
        }
        .item-label {
            flex-shrink: 0;
        }
        .item-value {
            min-width: 0;
            overflow: hidden;
            text-overflow: ellipsis;
            white-space: nowrap;
        }
    }
}
</style>
