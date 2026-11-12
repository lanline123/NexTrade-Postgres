<template>
    <div class="search-shop-panel">
        <div v-if="!initialLoading" class="shop-sort-tabs">
            <div class="tab-item" :class="{ active: activeTab === 'comprehensive' }" @click="handleTabChange('comprehensive')">
                {{ $t("综合") }}
            </div>
            <div class="tab-item" :class="{ active: activeTab === 'shopStar' }" @click="handleTabChange('shopStar')">
                {{ $t("评分") }}
            </div>
        </div>

        <template v-if="!initialLoading && total > 0">
            <div v-loading="loading" class="shop-list-container">
                <div v-for="item in filterState" :key="item.shopId" class="shop-row">
                    <ShopInfo :item="item" />
                    <ShopList :shop-id="item.shopId" :initial-list="item.listingProduct" />
                </div>
            </div>

            <div v-if="filterState.length > 0" class="el-page">
                <Pagination v-model:page="filterParams.page" v-model:size="filterParams.size" :total="total" @callback="loadFilter" />
            </div>
        </template>

        <template v-else-if="!initialLoading">
            <CommonAppEmpty :description="$t('暂无店铺数据')" />
        </template>

        <div v-else v-loading="true" class="search-shop-panel__loading"></div>
    </div>
</template>

<script lang="ts" setup>
// ===================== Vue 核心 =====================
import { reactive, ref, watch } from "vue";

// ===================== 组件导入 =====================
import { Pagination } from "~/components/list";
import ShopInfo from "~/pages/shop/components/ShopInfo.vue";
import ShopList from "~/pages/shop/components/ShopList.vue";

// ===================== API 导入 =====================
import { shopApi } from "./api";
import { errorMessage } from "~/types/member/error";

// ===================== 类型导入 =====================
import type { shopListItem } from "~/types/shop/shop.d";

const { t } = useI18n();

// ===================== Props =====================
const props = defineProps({
    keyword: {
        type: String,
        default: ""
    },
    active: {
        type: Boolean,
        default: false
    }
});

// ===================== 响应式状态 =====================
const filterState = ref<shopListItem[]>([]);
const loading = ref(false);
const initialLoading = ref(true);
const total = ref(0);
const activeTab = ref("comprehensive");
const hasLoaded = ref(false);

const filterParams = reactive({
    page: 1,
    size: 10,
    productNumber: 12,
    sortField: ""
});

// ===================== 数据获取方法 =====================
const loadFilter = async (isInitial = false): Promise<void> => {
    if (!props.active) return;

    if (isInitial) {
        initialLoading.value = true;
    } else {
        loading.value = true;
    }

    try {
        const result = await shopApi.getList({
            ...filterParams,
            keyword: props.keyword
        });
        filterState.value = result.records || [];
        total.value = result.total || 0;
        hasLoaded.value = true;
    } catch (error: unknown) {
        message.error(errorMessage(error) || t("加载失败"));
    } finally {
        if (isInitial) {
            initialLoading.value = false;
        } else {
            loading.value = false;
        }
    }
};

// ===================== 事件处理方法 =====================
const handleTabChange = (tab: string): void => {
    if (activeTab.value === tab) return;

    activeTab.value = tab;
    filterParams.page = 1;
    filterParams.sortField = tab === "shopStar" ? "shopStar" : "";
    loadFilter(false);
};

// ===================== 监听器 =====================
watch(
    () => [props.keyword, props.active],
    ([keyword, active], previousValue) => {
        if (!active) return;

        const [prevKeyword, prevActive] = previousValue ?? ["", false];
        const keywordChanged = keyword !== prevKeyword;
        const firstEnter = active && !prevActive;

        if (keywordChanged) {
            filterParams.page = 1;
        }

        if (!hasLoaded.value || keywordChanged || firstEnter) {
            loadFilter(true);
        }
    },
    {
        immediate: true
    }
);
</script>

<style scoped lang="less">
.search-shop-panel {
    width: 100%;
    margin-top: 12px;
}

.shop-sort-tabs {
    display: flex;
    align-items: center;
    gap: 30px;
    //margin-bottom: 20px;

    .tab-item {
        position: relative;
        padding: 8px 0;
        font-size: 16px;
        line-height: 20px;
        color: #666;
        cursor: pointer;
        transition: color 0.2s;
        user-select: none;

        &:hover,
        &.active {
            color: var(--general);
            font-weight: 700;
        }

        //&.active::after {
        //    content: "";
        //    position: absolute;
        //    left: 0;
        //    right: 0;
        //    bottom: -2px;
        //    height: 2px;
        //    border-radius: 999px;
        //    background: var(--general);
        //}
    }
}

.shop-list-container {
    display: flex;
    flex-direction: column;
    min-height: 200px;
    position: relative;
}

.shop-row {
    display: flex;
    align-items: stretch;
    //padding: 0 0 25px 0;
    background: #fff;
    border-bottom: 1px solid #eee;
    margin-bottom: 24px;

    &:last-child {
        border-bottom: none;
        margin-bottom: 0;
    }
}

.el-page {
    display: flex;
    justify-content: flex-end;
    padding: 20px 0;
}

.search-shop-panel__loading {
    min-height: 320px;
}
</style>
