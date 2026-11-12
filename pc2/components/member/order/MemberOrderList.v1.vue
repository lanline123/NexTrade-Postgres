<template>
    <div class="order-list">
        <OrderListFilter
            v-model:keyword="filterParams.keyword"
            :model-value="modelValue"
            :menu-type="menuType"
            :menu-type-list="menuTypeList"
            @change-type="changeType"
            @search="onSearch"
        ></OrderListFilter>
        <div class="order-title">
            <div class="ul">
                <template v-if="modelValue">
                    <div style="width: 23%">
                        <MemberOrderSelect v-model="filterParams.dateType" :selectList="orderTimeList" @loadFilter="loadFilter"></MemberOrderSelect>
                    </div>
                    <div style="width: 28%">{{ $t("商品名称") }}</div>
                </template>
                <template v-else>
                    <div style="width: 49%">{{ $t("商品名称") }}</div>
                </template>
                <div style="width: 6%">{{ $t("数量") }}</div>
                <div style="width: 7%"></div>
                <div style="width: 12%">{{ $t("金额") }}</div>
                <div style="width: 14%">{{ $t("交易状态") }}</div>
                <div style="width: 12%">{{ $t("操作") }}</div>
            </div>
        </div>
        <div v-loading="loading">
            <template v-if="total > 0">
                <OrderListCard
                    v-for="(item, index) in filterState"
                    :key="index"
                    :item="item"
                    @cancel-order="onCancelOrder"
                    @del-order="onDelOrder"
                    @confirm-receipt="onConfirmReceipt"
                    @re-buy="onReBuy"
                ></OrderListCard>
                <div class="el-page">
                    <Pagination v-model:page="filterParams.page" v-model:size="filterParams.size" :total="total" @callback="loadFilter" />
                </div>
            </template>
            <template v-else>
                <div class="no-result">
                    <div v-if="!loading">
                        <span class="sign sign1"></span>
                    </div>
                    <div v-if="!loading">
                        <ul>
                            <li>{{ $t("当前筛选下还没有任何订单哦") }}</li>
                            <li style="font-size: 12px">
                                {{ $t("现在就去") }}：
                                <NuxtLink class="font-color" to="/"> {{ $t("选购商品") }}</NuxtLink>
                            </li>
                        </ul>
                    </div>
                </div>
            </template>
        </div>
        <CancelOrderReasonDialog ref="cancelOrderReasonDialogRef" @success="onCancelSuccess" />
    </div>
</template>

<script lang="ts" setup>
import { onMounted, reactive, ref } from "vue";
import type { OrderFilterParams, OrderFilterState } from "~/types/user/order.d";
import { delOrder, getOrderList, getOrderNum, reBuy, confirmReceipt } from "~/api/user/order";
import { Pagination } from "~/components/list";
import { urlFormat } from "~/utils/format";
import { message, Modal } from "ant-design-vue";
import { useRoute } from "vue-router";
import { getJoiningActivityDetail } from "~/api/joining/joining";
import dayjs from "dayjs";
import OrderListFilter from "~/components/member/order/OrderListFilter.v1.vue";
import OrderListCard from "~/components/member/order/OrderListCard.v1.vue";
import CancelOrderReasonDialog from "~/components/member/order/CancelOrderReasonDialog.vue";

const { t } = useI18n();
defineProps({
    modelValue: {
        type: Number,
        default: 1
    }
});

const emit = defineEmits(["change"]);

// 基本参数定义
const filterState = ref(<OrderFilterState[]>[]);
const loading = ref<boolean>(true);
const total = ref<number>(0);
const filterParams = reactive<OrderFilterParams>({
    //初使化用于查询的参数
    page: 1,
    sortField: "addTime",
    sortOrder: "desc",
    keyword: "",
    payStatus: -1,
    dateType: 1
});
const route = useRoute();
const menuType = ref<string>("all");

interface menuType {
    type: string;
    value: string;
    num: number;
}

const menuTypeList = reactive<menuType[]>([
    { type: "all", value: "全部订单", num: 0 },
    { type: "awaitPay", value: "待支付", num: 0 },
    { type: "awaitShipping", value: "待发货", num: 0 },
    { type: "awaitReceived", value: "待收货", num: 0 },
    { type: "awaitComment", value: "待评价", num: 0 }
]);
const changeType = (type: string) => {
    filterParams.page = 1;

    filterParams.orderStatus = -1;
    filterParams.commentStatus = -1;
    if (type === "awaitPay") {
        filterParams.orderStatus = 0;
    } else if (type === "awaitShipping") {
        filterParams.orderStatus = 1;
    } else if (type === "awaitReceived") {
        filterParams.orderStatus = 2;
    } else if (type === "awaitComment") {
        filterParams.commentStatus = 0;
    }
    menuType.value = type;
    loadFilter();
};
const loadNum = async () => {
    loading.value = true;
    try {
        const result = await getOrderNum();
        menuTypeList.forEach((item: any) => {
            let temp: any = result;
            if (temp[item.type] !== undefined) {
                item.num = temp[item.type];
            }
        });
    } catch (error: any) {
        message.error(error.message);
    } finally {
        loading.value = false;
    }
};

// 是否次卡主订单
const isMainSubCardOrder = (item: any) => {
    return item.isCardOrder && Number(item.parentOrderSn) === 0;
};

const orderTimeList = reactive([
    { value: 0, label: "全部订单" },
    { value: 1, label: "最近三个月订单" },
    { value: 2, label: "最近六个月订单" },
    { value: 3, label: "最近一年内订单" }
]);
const cancelOrderReasonDialogRef = ref<{ open: (id: number) => void }>();
const onCancelOrder = (orderId: number) => {
    cancelOrderReasonDialogRef.value?.open(orderId);
};
const onCancelSuccess = async () => {
    await loadFilter();
    await loadNum();
};
const onDelOrder = async (orderId: number) => {
    Modal.confirm({
        title: t("确认要删除该订单吗"),
        cancelText: t("取消"),
        okText: t("确定"),
        maskClosable: true,
        centered: true,
        onOk: async () => {
            try {
                await delOrder({ id: orderId });
                message.success(t("删除成功"));
                await loadFilter();
                await loadNum();
            } catch (error: any) {
                message.error(error.message);
            } finally {
            }
        }
    });
};
// 再次购买
const onReBuy = async (item: any) => {
    const orderId = item.orderId;
    const productId = item.items[0]?.productId;
    const joinStatus = item.joinStatus;
    try {
        if (joinStatus) {
            const result = await getJoiningActivityDetail({ productId });
            if (result.endTime > dayjs().format("YYYY-MM-DD HH:mm:ss")) {
                navigateTo(urlFormat({ path: "product", sn: result.productSn }));
            } else {
                message.error(t("拼团已结束,可以按现价购买该商品"));
                await reBuy({ id: orderId });
                navigateTo(`/cart`);
            }
        } else if (isMainSubCardOrder(item)) {
            navigateTo(urlFormat({ path: "product", sn: item.items[0]?.productSn }));
        } else {
            await reBuy({ id: orderId });
            navigateTo(`/cart`);
        }
    } catch (error: any) {
        message.error(error.message);
    }
};

/**
 * 确认收货
 */
const onConfirmReceipt = async (orderId: number) => {
    Modal.confirm({
        title: t("确认要收货吗"),
        cancelText: t("取消"),
        okText: t("确定"),
        maskClosable: true,
        centered: true,
        onOk: async () => {
            try {
                const result = await confirmReceipt({ id: orderId });
                loadFilter();
                loadNum();
            } catch (error: any) {
                message.error(error.message);
            } finally {
            }
        }
    });
};

const loadFilter = async () => {
    loading.value = true;
    try {
        total.value = 0;
        const result = await getOrderList({ ...filterParams });
        filterState.value = result.records;
        total.value = result.total;
    } catch (error: any) {
        message.error(error.message);
    } finally {
        loading.value = false;
    }
};
const onSearch = () => {
    filterParams.page = 1;
    loadFilter();
};
onMounted(() => {
    loadNum().then(() => {
        changeType((route.query.type as string) || "all");
        // loadFilter();
    });
});
</script>

<style lang="less" scoped>
@import "/assets/css/member/member.v1";
</style>
