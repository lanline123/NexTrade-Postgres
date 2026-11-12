<template>
    <div class="search-page">
        <CommonPageHeader :is-new-search="true"></CommonPageHeader>
        <CommonHeader :title="filter.keyword ? `${filter.keyword} - 商品搜索` : '商品搜索'"></CommonHeader>
        <div class="page-body">
            <div class="container">
                <div class="top-filter-wrap">
                    <SearchTopFilter
                        ref="topFilterRef"
                        v-model:filter="filter"
                        v-model:filterSeleted="filterSeleted"
                        :query-params="queryParams"
                        :page-type="pageType"
                        :keyword="filter.keyword"
                        @change="onTopFilterChange"
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
                        <div v-if="searchStore.total > 0" class="search-tabs__right">
                            <div class="pageInfo">
                                <span class="noPage first-page" :class="{ disabled: filter.page == 1 }" @click="changeTopPage(-1)">
                                    <svg class="page-arrow page-arrow--prev" viewBox="0 0 24 24" width="16" height="16">
                                        <path d="M15 6l-6 6 6 6" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" />
                                    </svg>
                                </span>
                                <span class="page_number">
                                    <em class="page_current">{{ filter.page }}</em>
                                    <span class="page_separator">/</span>
                                    <span class="page_total">{{ pageCount }}</span>
                                </span>
                                <span class="noPage" :class="{ disabled: filter.page >= pageCount }" @click="changeTopPage(1)">
                                    <svg class="page-arrow" viewBox="0 0 24 24" width="16" height="16">
                                        <path d="M9 6l6 6-6 6" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" />
                                    </svg>
                                </span>
                            </div>
                        </div>
                    </div>
                </div>
                <div class="right-wrap">
                    <SearchSortFilter
                        ref="sortFilterRef"
                        v-model:filter="filter"
                        v-model:attrs="attrList"
                        :brand-options="brandOptions"
                        :filter-panel-data="filterPanelData"
                        @change="onFilterChange"
                        @panel-visible-change="onPanelVisibleChange"
                    />
                    <div class="goods-list-wrap" :class="{ 'goods-list-wrap--filter-panel-open': filterPanelVisible }">
                        <CommonCatGoodsList
                            v-model:queryParams="queryParams"
                            v-model:filter="filter"
                            :attrs="attrList"
                            :is-new-ui="true"
                            @change="onFilterChange"
                            @pageChange="onPageChange"
                            @filter-undo="onFilterUndo"
                        />
                    </div>
                </div>
            </div>
        </div>
        <CommonPageFooter></CommonPageFooter>
    </div>
</template>
<script lang="ts" setup>
/**
 * @description 商品搜索页 - 新版 UI 布局（顶部分类条 + 排序/筛选条 + 商品列表）
 * - 仅负责展示与组件编排，业务逻辑统一来自 useSearchPage 上下文
 */
import { ref, computed } from "vue";
import SearchTopFilter from "./SearchTopFilter.vue";
import SearchSortFilter from "./SearchSortFilter.vue";
import { useSearchPageContext } from "@/composables/useSearchPage";
import { useSearchStore } from "@/store/search";

const { pageType, filter, filterSeleted, attrList, queryParams, onFilterChange, onFilterUndo } = useSearchPageContext();

const searchStore = useSearchStore();

/** 每页商品数，与 CommonCatGoodsList 中 size 保持一致 */
const PAGE_SIZE = 25;
const pageCount = computed(() => Math.max(1, Math.ceil((searchStore.total || 0) / PAGE_SIZE)));

/** 内容标签（暂无店铺搜索，仅保留全部商品） */
const searchTabs = [{ label: "全部商品", value: "goods" as const }];
const activeContentTab = ref<"goods">("goods");

/** 顶部翻页器：在当前页基础上前进/后退一页 */
const changeTopPage = (step: number): void => {
    const nextPage = filter.page + step;
    if (nextPage < 1 || nextPage > pageCount.value) return;
    onFilterChange(nextPage);
    if (typeof window !== "undefined") {
        window.scrollTo({ top: 0, behavior: "smooth" });
    }
};

const topFilterRef = ref();
const sortFilterRef = ref<{ closeFilterPanel?: () => void } | null>(null);
const brandOptions = ref<any[]>([]);

/** 顶部分类条切换分类：先同步收起「全部筛选」浮层，再走正常筛选变更 */
const onTopFilterChange = (page?: number): void => {
    sortFilterRef.value?.closeFilterPanel?.();
    onFilterChange(page);
};
const filterPanelData = ref<{ brand: any[]; category: any[]; attrs: any[] }>({
    brand: [],
    category: [],
    attrs: []
});
const filterPanelVisible = ref(false);

const onBrandOptionsChange = (list: any[]): void => {
    brandOptions.value = list || [];
};

const onPanelDataChange = (data: { brand?: any[]; category?: any[]; attrs?: any[] }): void => {
    filterPanelData.value = {
        brand: data?.brand || [],
        category: data?.category || [],
        attrs: data?.attrs || []
    };
};

const onPanelVisibleChange = (visible: boolean): void => {
    filterPanelVisible.value = visible;
};

/** 分页变更滚动到顶部 */
const onPageChange = (): void => {
    if (typeof window !== "undefined") {
        window.scrollTo({ top: 0, behavior: "smooth" });
    }
};
</script>
<style lang="less" scoped>
.search-page {
    background-color: #fff;
}

.page-body {
    padding-top: 8px;
    padding-bottom: 0;
}

.container {
    display: flex;
    justify-content: space-between;
    flex-wrap: wrap;
    width: 100%;
    max-width: var(--pc-home-max-width, 1600px);
    min-width: var(--pc-home-min-width, 1264px);
    margin: 0 auto;
    padding: 0 var(--pc-home-padding-x, 38px);
    box-sizing: border-box;
    position: relative;

    .top-filter-wrap {
        width: 100%;
    }

    .right-wrap {
        width: 100%;
    }

    .goods-list-wrap {
        width: 100%;
        box-sizing: border-box;
        transition: padding-right 0.2s ease;

        &--filter-panel-open {
            padding-right: 384px;
        }
    }
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

        &.disabled {
            color: #ccc;
            cursor: not-allowed;
        }
    }

    .first-page {
        padding: 10px 5px 10px 10px;
    }
}
</style>
