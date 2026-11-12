<template>
    <PageHeaderNew :isWhite="true"></PageHeaderNew>
    <CommonHeader title="发票申请"></CommonHeader>
    <TopJumpBar tltle="发票申请"></TopJumpBar>
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
            <div class="finished-order-content">
                <div class="table-head">
                    <div class="item-op">{{ $t("商品名称") }}</div>
                    <div class="item-op">{{ $t("状态") }}</div>
                    <div class="item-op">{{ $t("操作") }}</div>
                </div>
                <div v-loading="loading" class="card-list">
                    <template v-if="total > 0">
                        <table v-for="item in filterState" class="custom-table" style="margin-top: 20px">
                            <thead class="table-header">
                                <tr>
                                    <th class="header-cell" colspan="3">
                                        {{ $t("订单号") }}：<span class="font-color">{{ item.orderSn }}</span> <span class="ml10"></span
                                        >{{ $t("下单时间") }}：{{ item.addTime }} <span class="ml10"></span>{{ $t("总金额") }}：
                                        <FormatPrice v-model="item.totalAmount" :fontStyle="{ color: 'var(--price)' }" :showText="false"></FormatPrice>
                                        ({{ $t("运费") }}：
                                        <FormatPrice v-model="item.shippingFee" :fontStyle="{ color: '#999' }" :showText="false"></FormatPrice>
                                        )
                                    </th>
                                </tr>
                            </thead>
                            <tbody>
                                <tr class="body-tr">
                                    <td>
                                        <NuxtLink
                                            v-for="img in item.items"
                                            :title="img.productName"
                                            :to="urlFormat({ path: 'product', seoLink: img.seoLink, sn: img.productSn })"
                                            class="nuxt"
                                            target="_blank"
                                        >
                                            <el-image :src="productImage(img)" class="image" loading="lazy" :alt="img.productName || ''" />
                                        </NuxtLink>
                                    </td>
                                    <td style="width: 110px">
                                        <template v-if="item.invoiceData">
                                            <span v-if="item.invoiceData.status === 0" style="color: green">{{ $t("待审核") }}</span>
                                            <span v-else-if="item.invoiceData.status === 1" style="color: green">{{ $t("通过") }}</span>
                                            <!-- <span v-else-if="item.invoiceData.status === 2" style="color: red">{{ $t("失败/作废") }}</span> -->
                                            <span v-else style="color: red"> {{ $t("失败/作废") }}</span>
                                        </template>
                                        <span v-else> {{ $t("可申请") }}</span>
                                    </td>
                                    <td style="width: 110px">
                                        <template v-if="item.invoiceData">
                                            <Info
                                                :params="{
                                                    orderId: item.orderId,
                                                    amount: item.balance,
                                                    orderSn: item.orderSn,
                                                    totalAmount: item.totalAmount,
                                                    type: 'info'
                                                }"
                                                @loadFilter="loadFilter"
                                            >
                                                <span class="font-color cup"> {{ $t("查看") }}</span>
                                            </Info>
                                            <template v-if="item.invoiceData.status === 2"
                                                >｜
                                                <Info
                                                    :params="{
                                                        orderId: item.orderId,
                                                        amount: item.balance,
                                                        orderSn: item.orderSn,
                                                        totalAmount: item.totalAmount,
                                                        type: 'reapply'
                                                    }"
                                                    @loadFilter="loadFilter"
                                                >
                                                    <span class="font-color cup"> {{ $t("重新申请") }}</span>
                                                </Info>
                                            </template>
                                        </template>
                                        <template v-else>
                                            <Info
                                                :params="{
                                                    orderId: item.orderId,
                                                    amount: item.balance,
                                                    orderSn: item.orderSn,
                                                    totalAmount: item.totalAmount,
                                                    type: 'apply'
                                                }"
                                                @loadFilter="loadFilter"
                                            >
                                                <el-button size="small" type="primary">{{ $t("发票申请") }}</el-button>
                                            </Info>
                                        </template>
                                    </td>
                                </tr>
                            </tbody>
                        </table>
                        <div class="el-page">
                            <Pagination v-model:page="filterParams.page" v-model:size="filterParams.size" :total="total" @callback="loadFilter" />
                        </div>
                    </template>
                    <template v-else>
                        <div class="no-result">
                            <div v-if="!loading">
                                <img class="empty-img" src="/assets/images/empty-box.png" alt="" />
                                <div class="empty-text">{{ $t("还没有可申请订单") }}</div>
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
 * 订单发票列表页
 * - 展示已开票/待开票订单列表
 * - 支持为订单申请开票
 * - 支持查看发票详情
 */
import { onMounted, reactive, ref } from "vue";
import { useRouter } from "vue-router";
import { getOrderList } from "~/api/user/order";
import type { OrderFilterParams, OrderFilterState } from "~/types/user/order";
import { Pagination } from "~/components/list";
import { delOrderInvoice } from "~/api/user/orderInvoice";
import { urlFormat } from "~/utils/format";
import Info from "~/components/invoice/InvoiceDialog.vue";
import PageHeaderNew from "~/components/common/PageHeaderNew.vue";
import TopJumpBar from "~/components/common/TopJumpBar.vue";

definePageMeta({
    middleware: "auth"
});

const { t } = useI18n();

// 基本参数定义
const filterState = ref(<OrderFilterState[]>[]);
const loading = ref<boolean>(true);
const total = ref<number>(0);
const filterParams = reactive<OrderFilterParams>({
    //初使化用于查询的参数
    page: 1,
    orderStatus: 5
});

const loadFilter = async () => {
    loading.value = true;
    try {
        const result = await getOrderList({ ...filterParams });
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

const reapply = async (value: any) => {
    try {
        loading.value = true;
        const result = await delOrderInvoice({ id: value });
        message.success(t("操作成功"));
        await loadFilter();
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
    { value: "发票申请", url: "/member/orderInvoice/list", size: 0 },
    { value: "增票资质", url: "/member/userInvoice/info", size: 0 }
]);
</script>
<style lang="less" scoped>
@import "/assets/css/member/member";

.finished-order-content {
    background: #fff;
    border: 0;

    .table-head {
        background: #f3f3f3;
        height: 42px;
        padding: 0 0 0 8px;
        display: flex;
        flex-direction: row;
        align-items: center;
        text-align: center;

        & > div:first-child {
            width: 100%;
        }

        .item-op {
            width: 150px;
            font-weight: 600;
        }
    }

    .custom-table {
        width: 100%;
        border-collapse: collapse;
        border: 1px solid #eee;

        .table-header {
            background-color: #f8f8f8;
        }

        .header-cell {
            color: #999;
            height: 40px;
            border: 1px solid #ddd;
            border-right: none;
            border-bottom: none;
            padding: 8px 10px;
            box-sizing: border-box;
            text-align: left;
        }

        .header-cell-first {
            border-left: none;
        }

        .header-cell-last {
            border-right: 1px solid #ddd;
        }

        .body-tr {
            box-sizing: border-box;
            text-align: center;

            .image {
                width: 42px;
                transition: opacity 0.2s linear;
                border: 0.8px solid #eee;
            }

            & > td:first-child {
                text-align: left;
                padding: 10px;
                display: flex;
                gap: 10px;
            }
        }
    }

    .cup {
        cursor: pointer;
    }
}
</style>
