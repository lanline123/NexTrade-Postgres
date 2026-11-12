<template>
    <!-- <CommonPageHeaderNew></CommonPageHeaderNew> -->
    <PageHeaderNew :isWhite="true"></PageHeaderNew>

    <CommonHeader></CommonHeader>
    <TopJumpBar tltle="售后申请"></TopJumpBar>

    <CommonPageContainer flex>
        <div class="menu">
            <MemberNav></MemberNav>
        </div>
        <div class="info-row">
            <div class="title-or-tabs">
                <div class="tag-and-input">
                    <div class="tig-tabs">
                        <MemberTopMenu :menuList="menuList"></MemberTopMenu>
                    </div>
                </div>
            </div>
            <div v-loading="loading" class="return-content">
                <template v-if="total > 0">
                    <table class="custom-table">
                        <thead class="table-header">
                            <tr>
                                <th class="header-cell header-cell-first" style="width: 200px">{{ $t("订单编号") }}</th>
                                <th class="header-cell">{{ $t("订单商品") }}</th>
                                <th class="header-cell" style="width: 140px">{{ $t("发货时间") }}</th>
                            </tr>
                        </thead>
                        <tbody>
                            <tr v-for="(item, index) in filterState" :key="index">
                                <td>{{ item.orderSn }}</td>
                                <td>
                                    <div class="product-list">
                                        <div v-for="it in item.items">
                                            <el-image :src="imageFormat(it.productPicThumb)" :alt="it.productName || ''" class="img" />
                                            <template v-if="it.aftersaleFlag == 1 && it.toAftersalses">
                                                <NuxtLink
                                                    v-if="it.aftersaleFlag == 1 && it.toAftersalses"
                                                    :to="'/member/return/Info?orderId=' + item.orderId + '&itemId=' + it.itemId"
                                                    ><el-button type="primary">{{ $t("申请") }}</el-button>
                                                </NuxtLink>
                                            </template>
                                            <template v-if="it.aftersaleFlag != 1">
                                                <NuxtLink :to="'/member/return/logInfo?id=' + it.aftersalesItem.aftersaleId"
                                                    ><el-button>{{ $t("详情") }}</el-button>
                                                </NuxtLink>
                                            </template>
                                        </div>
                                    </div>
                                </td>
                                <td>{{ item.shippingTime ? item.shippingTime : $t("未发货") }}</td>
                            </tr>
                        </tbody>
                    </table>
                    <div class="el-page">
                        <Pagination v-model:page="filterParams.page" v-model:size="filterParams.size" :total="total" @callback="loadFilter" />
                    </div>
                </template>
                <template v-else>
                    <div class="no-result">
                        <div v-if="!loading" style="display: flex; flex-direction: column; align-items: center">
                            <img class="empty-img" src="/assets/images/empty-box.png" alt="" />
                            <div class="empty-text">{{ $t("暂无可申请退换货订单") }}</div>
                        </div>
                    </div>
                </template>
            </div>
        </div>
    </CommonPageContainer>

    <CommonPageFooter></CommonPageFooter>
</template>
<script lang="ts" setup>
/**
 * @description 退换货列表页面
 * - 展示用户所有退换货申请记录
 * - 显示退换货状态、申请时间、商品信息
 * - 支持分页浏览和查看详情
 */
import { reactive, ref } from "vue";
import { useRouter } from "vue-router";
import type { AfterSalesFilterParams, AfterSalesFilterState } from "~/types/user/afterSales";
import { getAfterSalesList } from "~/api/user/afterSales";
import PageHeaderNew from "~/components/common/PageHeaderNew.vue";
import TopJumpBar from "~/components/common/TopJumpBar.vue";

import { Pagination } from "~/components/list";
definePageMeta({
    middleware: "auth"
});

// 基本参数定义
const filterState = ref(<AfterSalesFilterState[]>[]);
const loading = ref<boolean>(true);
const total = ref<number>(0);
const filterParams = reactive<AfterSalesFilterParams>({
    //初使化用于查询的参数
    page: 1
});
const loadFilter = async () => {
    loading.value = true;
    try {
        const result = await getAfterSalesList({ ...filterParams });
        filterState.value = result.records;
        total.value = result.total;
    } catch (error: any) {
        message.error(error.message);
    } finally {
        loading.value = false;
    }
};
onMounted(() => {
    loadFilter();
});

const router = useRouter();
const splitStr = (str: string) => {
    return str.split("?")[0];
};
const menuList = reactive<any>([
    { value: "售后申请", url: "/member/return/list", size: 0 },
    { value: "申请记录", url: "/member/return/log", size: 0 }
]);
</script>
<style lang="less" scoped>
@import "/assets/css/member/member";

.return-content {
    background: #fff;
    border: 0;
    font-size: 14px;
    // padding: 20px;

    .custom-table {
        width: 100%;
        border-collapse: collapse;
        border: 1px solid #ddd;
    }

    .table-header {
        height: 50px;
        background-color: #f5f6fa;
    }

    .header-cell {
        border: 1px solid #ddd;
        border-right: none;
        padding: 8px 10px;
        text-align: left;
    }

    .header-cell-first {
        border-left: none;
    }

    .header-cell-last {
        border-right: 1px solid #ddd;
    }

    table td,
    .header-cell {
        border: 1px solid #ddd;
        padding: 8px 10px;
        text-align: left;
    }

    .table-footer td {
        text-align: left;
        background-color: #fcfcfc;
        border-top: 1px solid #ddd;
    }

    .product-list {
        display: flex;
        flex-direction: row;
        width: 100%;
        gap: 20px;

        flex-wrap: wrap;

        & > div {
            display: flex;
            flex-direction: column;
            justify-content: center;
            align-items: center;
            gap: 10px;
        }

        .img {
            width: 60px;
            height: 60px;
            border: 0.8px solid #eee;
            border-radius: 4px;
        }
    }
}
</style>
