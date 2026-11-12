<template>
    <div class="shop-search-page">
        <CommonTopBar :colorType="colorType" :pageType="pageType" :wide="true"></CommonTopBar>
        <!-- <PcHeaderTopBarRuntime :styleType="headerStyleType" :isWhite="true" :config="decorateRuntimeStore.pcHeaderConfig || {}" :wide="true"></PcHeaderTopBarRuntime> -->
        <ShopRefHeader :shopInfo="shopInfo" @collect="handleCollect" @kefu="handleKefu" />

        <div class="container main-content">
            <div class="right-content" v-loading="loadingProduct">
                <div class="shop-sort-bar">
                    <span class="sort-link" :class="{ cur_sort: filter.sortField === '' }" @click="changeSort('')">{{ $t("综合") }}</span>
                    <span class="sort-link" :class="{ cur_sort: filter.sortField === 'sale' }" @click="changeSort('sale')">{{ $t("销量") }}</span>
                    <span class="sort-link" :class="{ cur_sort: filter.sortField === 'time' }" @click="changeSort('time')">{{ $t("新品") }}</span>
                    <span class="sort-link" :class="{ cur_sort: filter.sortField === 'price' }" @click="changeSort('price')">
                        {{ $t("价格") }}
                        <span class="sort-arrow">
                            <i
                                class="iconfont-pc icon-shangjiantou1"
                                :class="{ active: filter.sortField === 'price' && filter.sortOrder === 'asc' }"
                            ></i>
                            <i
                                class="iconfont-pc icon-xiajiantou1"
                                :class="{ active: filter.sortField === 'price' && filter.sortOrder === 'desc' }"
                            ></i>
                        </span>
                    </span>
                </div>
                <CatGoodsList
                    v-model:queryParams="queryParams"
                    v-model:filter="filter"
                    v-model:loadingProduct="loadingProduct"
                    :attrs="attrs"
                    @filter-undo="onCatGoodsFilterUndo"
                />
            </div>
        </div>

        <CommonPageFooter :type="2" bg="#fff" />
    </div>
</template>

<script setup lang="ts">
/**
 * @description 店铺内搜索页 - 新版 UI
 * - 布局与 ShopDefaultPage 一致，不含左侧分类菜单
 */
import { ref, reactive, onMounted, watch, computed } from "vue";
import { useRoute, useRouter } from "vue-router";
import { getShopDetail, getShopCollect } from "~/api/product/product";
import { message } from "ant-design-vue";
import ShopRefHeader from "~/pages/shop/src/ShopRefHeader.vue";
import CatGoodsList from "~/pages/shop/src/CatGoodsList.vue";
import type { QueryParams } from "~/types/search/search";
import { useCommonStore } from "@/store/common";
import { useDecorateRuntimeStore } from "@/store/decorateRuntime";
import { safeOpen, isOverseas } from "@/utils/util";
import PcHeaderTopBarRuntime from "~/decorate/components/pc/pcHeaderRuntime/components/PcHeaderTopBarRuntime.vue";

const { t } = useI18n();
const decorateRuntimeStore = useDecorateRuntimeStore();

const commonStore = useCommonStore();
const pageType = computed(() => {
    return commonStore.decoratePageConfig?.headerStyle;
});
const colorType = computed(() => {
    return commonStore.decoratePageConfig?.colorStyle;
});

// 新版头部顶部条风格：跟随后台 PC 头部装修配置，无配置时按 overseas 兜底
const headerStyleType = computed<1 | 2>(() => {
    const raw = decorateRuntimeStore.pcHeaderConfig?.styleType;
    if (raw === 1 || raw === 2) return raw;
    return isOverseas() ? 2 : 1;
});

const route = useRoute();
const router = useRouter();
const shopId = ref(Number(route.params.id) || Number(route.query.shopId) || 0);

const shopInfo = ref<any>({});
const loadingProduct = ref(false);
const attrs = ref([]);

const filter = reactive<any>({
    page: 1,
    keyword: "",
    shopId: shopId.value,
    sortField: "",
    sortOrder: ""
});

const queryParams = computed(() => {
    const query: any = {
        shopId: shopId.value
    };
    if (filter.page > 1) {
        query.page = filter.page;
    }
    if (filter.keyword) {
        query.keyword = filter.keyword;
    }
    if (filter.sortField !== "" && ["sale", "price", "time"].includes(filter.sortField)) {
        query.sort = filter.sortField;
    }
    if (filter.sortOrder !== "" && ["desc", "asc"].includes(filter.sortOrder)) {
        query.order = filter.sortOrder;
    }

    if (route.query.categoryId) {
        query.shopCategoryId = Number(route.query.categoryId);
    }

    return query as QueryParams;
});

const changeSort = (sort: string) => {
    if (sort === "price") {
        filter.sortField = "price";
        filter.sortOrder = filter.sortOrder === "asc" ? "desc" : "asc";
    } else {
        filter.sortField = sort;
        filter.sortOrder = "";
    }
    filter.page = 1;
};

const onCatGoodsFilterUndo = () => {
    if (typeof window !== "undefined" && window.history.length > 1) {
        router.back();
        return;
    }
    router.replace({
        path: router.currentRoute.value.path,
        query: {}
    });
};

const fetchShopInfo = async () => {
    try {
        const res = await getShopDetail(shopId.value);
        shopInfo.value = res;
    } catch (error) {
        console.error(error);
    }
};

const handleCollect = async () => {
    try {
        await getShopCollect(shopId.value);
        message.success(shopInfo.value.collectShop ? t("取消关注成功") : t("关注成功"));
        fetchShopInfo();
    } catch (error: any) {
        message.error(error.message || t("操作失败"));
    }
};

const handleKefu = () => {
    if (shopInfo.value.kefuLink) {
        safeOpen(shopInfo.value.kefuLink, "newWindow", "width=800,height=600,scrollbars=yes,resizable=yes");
    } else {
        safeOpen(`/chat?shopId=${shopId.value}`, "newWindow", "width=800,height=600,scrollbars=yes,resizable=yes");
    }
};

watch(
    () => route.query,
    (newQuery) => {
        if (newQuery.keyword) {
            filter.keyword = String(newQuery.keyword);
        }
    },
    { deep: true, immediate: true }
);

onMounted(() => {
    if (shopId.value) {
        fetchShopInfo();
    }
});
</script>

<style scoped lang="less">
.shop-search-page {
    background-color: #f5f5f5;
    min-height: 100vh;
}

.container {
    max-width: 1700px;
    width: 100%;
    margin: 0 auto;
    padding-left: 50px;
    padding-right: 50px;
    box-sizing: border-box;
}

.main-content {
    display: flex;
    align-items: flex-start;
    gap: 16px;
    padding-bottom: 40px;

    .right-content {
        flex: 1;
        width: 100%;
        background: #fff;
        padding: 16px;
        border-radius: 16px;
        overflow: hidden;

        .shop-sort-bar {
            display: flex;
            align-items: center;
            gap: 8px;
            margin-bottom: 16px;

            .sort-link {
                display: inline-flex;
                align-items: center;
                height: 31px;
                box-sizing: border-box;
                padding: 9px 12px;
                font-size: 14px;
                font-weight: 400;
                color: #333;
                background: #f5f5f5;
                border-radius: 4px;
                cursor: pointer;
                transition: all 0.2s;

                .sort-arrow {
                    display: inline-flex;
                    flex-direction: column;
                    align-items: center;
                    justify-content: center;
                    gap: 2px;
                    margin-left: 6px;
                    line-height: 1;

                    .iconfont-pc {
                        font-size: 10px;
                        color: #ccc;
                        height: 6px;
                        line-height: 6px;

                        &.active {
                            color: var(--general);
                        }
                    }
                }

                &:hover {
                    color: var(--general);
                }

                &.cur_sort {
                    color: var(--vice-text);
                    background-color: var(--primary-light-9);

                    &:hover {
                        color: var(--vice-text);
                    }

                    .sort-arrow .iconfont-pc {
                        color: #ccc;

                        &.active {
                            color: var(--vice-text);
                        }
                    }
                }
            }
        }

        :deep(.goods-list-c) {
            .wrap {
                display: flex;
                flex-wrap: wrap;
                gap: 16px;

                .item {
                    width: calc((100% - 80px) / 6);
                    margin: 0;
                    padding: 0;

                    &:nth-child(6n),
                    &:nth-child(5n),
                    &:nth-child(4n) {
                        margin: 0;
                    }

                    @media (max-width: 1649px) {
                        width: calc((100% - 64px) / 5);
                    }

                    @media (max-width: 1379px) {
                        width: calc((100% - 48px) / 4);
                    }
                    border: none;
                    transition: all 0.3s;
                    box-sizing: border-box;

                    &:hover {
                        border-color: transparent;
                        box-shadow: none;
                        transform: translateY(-2px);
                    }

                    .pic {
                        width: 100%;
                        height: auto;
                        aspect-ratio: 1;
                        margin-bottom: 8px;
                        border-radius: 8px;
                        overflow: hidden;

                        a {
                            width: 100%;
                            height: 100%;
                            display: block;
                        }

                        .el-image {
                            width: 100%;
                            height: 100%;
                            border-radius: 8px;
                        }
                    }

                    .info {
                        min-height: auto;

                        .name {
                            height: auto;
                            min-height: 40px;
                            line-height: 20px;
                            font-size: 13px;
                            margin-bottom: 8px;
                            padding: 0;
                            overflow: visible;

                            a {
                                display: -webkit-box;
                                -webkit-line-clamp: 2;
                                line-clamp: 2;
                                -webkit-box-orient: vertical;
                                overflow: hidden;
                                text-overflow: ellipsis;
                                word-break: break-word;
                            }
                        }

                        .price {
                            margin-bottom: 8px;

                            .num {
                                font-size: 16px;
                            }
                        }
                    }
                }
            }
        }
    }
}
</style>
