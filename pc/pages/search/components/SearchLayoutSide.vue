<template>
    <div class="container container--filter-side" :style="layoutCssVars">
        <aside class="left-wrap">
            <FilterSide
                v-model:filter="filter"
                v-model:filter-seleted="filterSeleted"
                v-model:filter-loading="filterLoading"
                :query-params="queryParams"
                :page-type="pageType"
                :show-price-range="showPriceRange"
                @change="onChange"
            />
        </aside>
        <main class="right-wrap">
            <SortFilterSide v-model:filter="filter" @change="onChange" />
            <div class="goods-list-wrap">
                <CatGoodsList
                    v-model:filter-loading="filterLoading"
                    v-model:filter="filter"
                    :query-params="queryParams"
                    :filter-seleted="filterSeleted"
                    :search-config="searchConfig"
                    :fixed-name-two-lines="true"
                    @change="onChange"
                    @list-length="(len: number) => emit('listLength', len)"
                    @load-finished="() => emit('loadFinished')"
                />
            </div>
        </main>
    </div>
</template>

<script lang="ts" setup>
import { computed, type PropType, type Ref } from "vue";

import FilterSide from "./FilterSide.vue";
import SortFilterSide from "./SortFilterSide.vue";
import CatGoodsList from "./CatGoodsList.vue";

import type { SearchFilter, filterSeleted, QueryParams } from "./types.d";
import type { PcSearchModuleData } from "~/store/decorateRuntime";

const props = defineProps({
    searchConfig: {
        type: Object as PropType<Partial<PcSearchModuleData>>,
        default: () => ({})
    },
    queryParams: {
        type: Object as PropType<QueryParams>,
        default: () => ({})
    },
    pageType: {
        type: String,
        default: "/search"
    }
});

const emit = defineEmits(["change", "listLength", "loadFinished"]);

const filter = defineModel<SearchFilter>("filter", { default: () => ({ brandIds: [], attrs: [] }) }) as Ref<SearchFilter>;
const filterSeleted = defineModel<filterSeleted | undefined>("filterSeleted");
const filterLoading = defineModel<boolean>("filterLoading", { default: false });

const showPriceRange = computed(() => props.searchConfig.showPriceRange !== false);

// 左侧布局：按模板真实列数展示（一行五个 / 一行六个）
const layoutCssVars = computed(() => {
    const filterTemplate = Number(props.searchConfig.filterTemplate ?? 2);
    const columns = filterTemplate === 2 ? 6 : 5;
    return { "--search-grid-columns": String(columns) } as Record<string, string>;
});

const onChange = (page?: number): void => {
    emit("change", page);
};
</script>

<style lang="less" scoped>
.container {
    display: flex;
    gap: 20px;
    width: 100%;
    max-width: var(--pc-home-max-width, 1600px);
    min-width: var(--pc-home-min-width, 1264px);
    padding: 0 var(--pc-home-padding-x, 38px);
    box-sizing: border-box;
    margin-top: 30px;

    .left-wrap {
        flex-shrink: 0;
        width: 240px;
    }

    .right-wrap {
        flex: 1;
        min-width: 0;
    }

    .goods-list-wrap {
        width: 100%;
        box-sizing: border-box;
    }
}
</style>
