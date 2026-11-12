<template>
    <PageHeaderNew :isWhite="true"></PageHeaderNew>
    <CommonHeader title="心愿单"></CommonHeader>
    <TopJumpBar tltle="心愿单"></TopJumpBar>
    <CommonPageContainer flex>
        <div class="menu">
            <MemberNav></MemberNav>
        </div>
        <div class="info-row collect-product">
            <div class="title-or-tabs">
                <span class="card-title">{{ $t("心愿单") }}</span>
            </div>
            <div class="page-body">
                <div ref="gridRef" v-loading="loading" class="collect-list-content" :style="{ '--collect-columns': columns }">
                    <template v-if="total > 0">
                        <template v-for="(item, index) in filterState" :key="index">
                            <ProductCard :product="item" :is-more="true" :product-style="styleMap[item.productId]" @delete="handleDel"></ProductCard>
                        </template>
                        <div class="el-page">
                            <Pagination v-model:page="filterParams.page" v-model:size="filterParams.size" :total="total" @callback="loadFilter" />
                        </div>
                    </template>
                    <template v-else>
                        <div class="no-result">
                            <div v-if="!loading" class="empty">
                                <img class="empty-img" src="/assets/images/empty-box.png" alt="" />
                                <div class="empty-text">{{ $t("您还没有收藏的商品") }}</div>
                            </div>
                        </div>
                    </template>
                </div>
            </div>
        </div>
    </CommonPageContainer>
    <CommonPageFooter></CommonPageFooter>
</template>
<script lang="ts" setup>
/**
 * @description 商品收藏列表页面
 * - 展示用户收藏的所有商品，包括商品图片、名称、价格
 * - 支持取消收藏、直接购买（加入购物车）操作
 * - 提供关键词搜索功能筛选收藏商品
 * - 支持分页加载收藏列表
 */
import { reactive, ref } from "vue";
import type { CollectProductFilterParams, CollectProductFilterState } from "~/types/user/collectProduct";
import { delCollectProduct, getCollectProductList } from "~/api/user/collectProduct";
import { urlFormat } from "~/utils/format";
import { Pagination } from "~/components/list";
import Buy from "~/components/product/Buy.vue";
import { useRouter } from "vue-router";
import PageHeaderNew from "~/components/common/PageHeaderNew.vue";
import TopJumpBar from "~/components/common/TopJumpBar.vue";
import { useProductStyle } from "~/hooks";
import { useResponsiveGridColumns } from "~/hooks/useResponsiveGridColumns";
definePageMeta({
    middleware: "auth"
});
const filterState = ref(<CollectProductFilterState[]>[]);
const { styleMap } = useProductStyle(filterState);
const { containerRef: gridRef, columns } = useResponsiveGridColumns({ minCardWidth: 240, gap: 8, minColumns: 4, maxColumns: 6 });
const loading = ref<boolean>(true);
const total = ref<number>(0);
const filterParams = reactive<CollectProductFilterParams>({
    //初使化用于查询的参数
    page: 1,
    keyword: ""
});
const { t } = useI18n();
let requestGeneration = 0;

const loadFilter = async () => {
    const currentRequest = ++requestGeneration;
    loading.value = true;
    try {
        const result = await getCollectProductList({ ...filterParams });
        if (currentRequest !== requestGeneration) return;
        filterState.value = result.records;
        total.value = result.total;
    } catch (error: any) {
        message.error(error.message);
    } finally {
        if (currentRequest === requestGeneration) loading.value = false;
    }
};
loadFilter();


const handleDel = (product: any) => {
    deleteASiteCollection(product?.productId);
};

const deleteASiteCollection = async (value: number) => {
    try {
        loading.value = true;
        const result = await delCollectProduct({ id: value });
        await loadFilter();
        message.success(t("取消收藏成功"));
    } catch (error: any) {
        message.error(error.message);
    } finally {
        loading.value = false;
    }
};
const router = useRouter();
const splitStr = (str: string) => {
    return str.split("?")[0];
};
const menuList = reactive<any>([
    { value: "收藏的商品", url: "/member/collectProduct/list", size: 0 },
    { value: "收藏的店铺", url: "/member/collectShop/list", size: 0 }
]);
</script>

<style lang="less" scoped>
@import "/assets/css/member/member";

.empty {
    display: flex;
    flex-direction: column;
    align-items: center;
}


.collect-product {
    width: 1008px !important;
    .title-or-tabs {
        .tag-and-input {
            display: flex;
            flex-direction: row;
            justify-content: space-between;
            width: 100%;
            margin-left: 20px;

            .tig-tabs {
                & > div {
                    color: #333333;
                    font-size: 16px;
                    font-weight: 400;
                    letter-spacing: 0;
                    display: inline-block;
                }
            }

            .tig-font-style {
                font-size: 14px;
            }

            .image {
                margin-bottom: 20px;
            }

            .input {
                display: flex;
                align-items: center;
            }
        }
    }
    .page-body {
        margin-top: 24px;
        .collect-list-content {
            display: grid;
            grid-template-columns: repeat(var(--collect-columns, 4), minmax(0, 1fr));
            gap: 8px;
            align-content: start;

            // 分页/空状态占满整行，不参与商品列
            .el-page,
            .no-result {
                grid-column: 1 / -1;
            }

            // 卡片填满弹性列宽、图片随列宽保持正方形（覆盖 ProductCard 的固定 240px）
            :deep(.product-card-container) {
                width: 100%;
            }
            :deep(.product-card-container .pro_pic) {
                width: 100%;
                height: auto;
                aspect-ratio: 1 / 1;
            }
        }
    }
}
</style>
