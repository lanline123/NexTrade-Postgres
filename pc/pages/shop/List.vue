<!--
  店铺街/店铺列表页面（行卡片布局，统一标准版/跨境版）
  - 展示平台所有入驻店铺，行卡片：左侧店铺信息卡 + 右侧热销商品横向滚动
  - 综合/评分 排序 Tab（sortField）与 keyword 搜索
  - 保留 o2o 站头/脚 chrome 与 SEO（行卡片样式统一自跨境版）
-->
<template>
    <div class="shop-list-page">
        <CommonPageHeader defaultSearchType="shop"></CommonPageHeader>
        <CommonHeader :title="$t('店铺街')"></CommonHeader>
        <CommonPageContainer>
            <div class="filter-tabs" v-if="!initialLoading">
                <div class="tab-item" :class="{ active: activeTab === 'comprehensive' }" @click="handleTabChange('comprehensive')">{{ $t("综合") }}</div>
                <div class="tab-item" :class="{ active: activeTab === 'shopStar' }" @click="handleTabChange('shopStar')">{{ $t("评分") }}</div>
            </div>

            <div
                v-loading="initialLoading || loading"
                :element-loading-text="$t('加载中...')"
                element-loading-background="rgba(255, 255, 255, 0.8)"
                class="shop-list-wrapper"
                :style="{ minHeight: initialLoading || loading ? '400px' : 'auto' }"
            >
                <template v-if="!initialLoading && total > 0">
                    <div class="shop-list-container">
                        <div class="shop-row" v-for="item in filterState" :key="item.shopId">
                            <ShopInfo :item="item" />
                            <ShopList :initial-list="item.listingProduct" />
                        </div>
                    </div>

                    <div class="el-page" v-if="filterState.length > 0">
                        <Pagination v-model:page="filterParams.page" v-model:size="filterParams.size" :total="total" @callback="loadFilter" />
                    </div>
                </template>

                <template v-if="!initialLoading && !loading && total === 0">
                    <el-empty :description="$t('暂无店铺数据')" />
                </template>
            </div>
        </CommonPageContainer>
        <CommonPageFooter :type="2" bg="#fff"></CommonPageFooter>
    </div>
</template>

<script setup lang="ts">
/**
 * 店铺街/店铺列表页面
 * - 调用 getShopList 获取店铺列表，支持分页
 * - 支持 综合/评分 排序（sortField）与 keyword 搜索
 * - 行卡片布局：ShopInfo（店铺信息卡）+ ShopList（商品横向滚动）
 */
import { reactive, ref, computed } from "vue";
import { Pagination } from "~/components/list";
import { groupListParams } from "~/utils/shop-group";
import { getShopList } from "~/api/shop/shop";
import type { shopListItem } from "~/types/shop/shop.d";
import ShopInfo from "./src/ShopInfo.vue";
import ShopList from "./src/ShopList.vue";
import { useCommonStore } from "@/store/common";
import { message } from "ant-design-vue";
import { isMerchant } from "~/utils/util";

const route = useRoute();
const commonStore = useCommonStore();
const { t } = useI18n();
const merchantEnabled = isMerchant();

if (!merchantEnabled) {
    navigateTo("/404", { replace: true });
}

useSeo({
    title: computed(() => (route.query.keyword ? `${route.query.keyword} - ${t("店铺街")}` : t("店铺街"))),
    description: computed(() => commonStore.shopDesc || t("店铺街")),
    keywords: computed(() => commonStore.shopKeywords || t("店铺街")),
    canonical: computed(() => "/shop/list"),
    robots: computed(() => (route.query.keyword ? "noindex,follow" : "index,follow"))
});

const filterState = ref<shopListItem[]>([]);
const loading = ref<boolean>(false);
const initialLoading = ref<boolean>(true);
const total = ref<number>(0);
const activeTab = ref<string>("comprehensive");
const filterParams = reactive({
    page: 1,
    size: 10,
    productNumber: 12,
    sortField: "",
    keyword: ""
});

let loadSequence = 0;
const loadFilter = async (isInitial: boolean = false) => {
    const sequence = ++loadSequence;
    filterParams.keyword = route.query.keyword ? String(route.query.keyword) : "";
    if (isInitial) {
        initialLoading.value = true;
    } else {
        loading.value = true;
    }
    try {
        const result = await getShopList({ ...filterParams, ...groupListParams(route.query, 1) });
        if (sequence !== loadSequence) return;
        filterState.value = result.records;
        total.value = result.total;
    } catch (error: any) {
        if (sequence !== loadSequence) return;
        filterState.value = [];
        total.value = 0;
        message.error(error.message);
    } finally {
        if (sequence === loadSequence) {
            loading.value = false;
            initialLoading.value = false;
        }
    }
};

const handleTabChange = (tab: string) => {
    if (activeTab.value === tab) return;
    activeTab.value = tab;
    filterParams.page = 1;
    filterParams.sortField = tab === "shopStar" ? "shopStar" : "";
    window.scrollTo({ top: 0, behavior: "smooth" });
    loadFilter(false);
};

watch(
    () => [route.query.keyword, route.query.groupIds],
    () => {
        filterParams.page = 1;
        loadFilter(true);
    }
);

if (merchantEnabled) {
    loadFilter(true);
}
</script>

<style scoped lang="less">
.shop-list-page {
    background-color: #fff;
    min-height: 100vh;
}

.container {
    margin: 0 auto;
    padding-top: 0px;
    padding-bottom: 40px;
}

.filter-tabs {
    display: flex;
    align-items: center;
    padding: 20px 0 0;
    gap: 30px;

    .tab-item {
        font-size: 14px;
        color: #666;
        cursor: pointer;
        padding: 8px 0;
        position: relative;
        transition: color 0.3s;
        user-select: none;

        &:hover {
            color: var(--general);
        }

        &.active {
            color: var(--general);
            font-weight: bold;
        }
    }
}

.shop-list-wrapper {
    position: relative;
}

.shop-list-container {
    display: flex;
    flex-direction: column;
}

.shop-row {
    background: #fff;
    display: flex;
    align-items: stretch;
    padding: 15px 0;
    border-bottom: 1px solid #ddd;

    &:last-child {
        border-bottom: none;
    }
}

.el-page {
    display: flex;
    justify-content: flex-end;
    padding: 20px 0;
}
</style>
