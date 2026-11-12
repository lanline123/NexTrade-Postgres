<template>
    <div class="title-or-tabs">
        <span>{{ $t("我的订单") }}</span>
        <div class="input">
            <el-input style="height: 40px; width: 260px; --el-input-border-radius: 4px; border-radius: 4px" v-model="filterParams.keyword" :placeholder="$t('收货人名称或订单号')" type="text" />
            <el-button style="height: 40px; width: 76px; border-radius: 4px" class="ml10" @click="onSearch()">{{ $t("搜索") }}</el-button>
        </div>
    </div>
    <div class="order-list">
        <div v-if="modelValue" class="tag-and-input">
            <div class="tig-tabs">
                <div v-for="item in visibleMenuTypeList" :class="menuType == item.type ? 'activeert' : ''" class="tab" @click="changeType(item.type)">
                    <p>
                        {{ $t(item.value) }}
                        <span v-if="item.num">{{ item.num }}</span>
                    </p>
                </div>
            </div>
        </div>
        <div v-loading="loading" style="padding-top: 12px">
            <template v-if="total > 0">
                <div v-for="(item, index) in filterState" :key="index" class="memberOrder">
                    <div class="memberOrder_top">
                        <div class="memberOrder_top_left">
                            <span>{{ item.orderStatusName }}</span>
                            <span v-if="item.cancelRemark?.trim()" class="cancel-reason">
                                {{ $t("取消原因") }}：{{ item.cancelRemark }}
                            </span>
                        </div>
                        <div>
                            <span class="black">
                                <NuxtLink class="flex_cc" :to="'/member/order/info?id=' + item.orderId" target="_blank">
                                    {{ $t("订单详情") }}
                                    <SvgIcon class="icon" name="rightIcon" width="16px" height="16px"></SvgIcon>
                                </NuxtLink>
                            </span>
                        </div>
                    </div>
                    <div class="memberOrder_middle">
                        <div class="memberOrder_middle_left">
                            <div class="fs_14">
                                {{ $t("订单号") }}：<NuxtLink :to="'/member/order/info?id=' + item.orderId" target="_blank"
                                    ><span>{{ item.orderSn }}</span></NuxtLink
                                >
                            </div>
                        </div>
                        <div class="fs_14">
                            {{ $t("共{0}件", [item.items?.reduce((sum, p) => sum + Number(p.quantity), 0)]) }}: 
                            <FormatPrice
                                v-model="item.totalAmount"
                                :showText="false"
                                :priceStyle="{ fontSize: '16px', color: '#1a1a1a', fontWeight: 'bold' }"
                                :currencyStyle="{ fontSize: '14px' }"
                            ></FormatPrice>
                        </div>
                    </div>
                    <div class="memberOrder_shop" v-for="(product, pIndex) in item.items">
                        <NuxtLink
                            :title="product.productName"
                            :to="urlFormat({ path: 'product', seoLink: product.seoLink, sn: product.productSn, shopId: item.shopId > 0 ? item.shopId : undefined })"
                            class="nuxt"
                            target="_blank"
                        >
                            <div class="flex">
                                <el-image
                                    :src="productSkuImage(product)"
                                    loading="lazy"
                                    style="width: 60px; height: 60px; cursor: pointer; transition: opacity 0.2s linear; flex-shrink: 0; border-radius: 4px"
                                    :alt="product.productName || ''"
                                />
                                <div class="memberOrder_shop_text">
                                    <div class="text-ellipsis">
                                        <span v-if="item.isPresale" class="presale-text">{{ $t("预售") }}</span>
                                        <span v-if="item.joinStatus" class="joining-tag">{{ $t("拼团") }}</span>
                                        {{ product.productName }}
                                    </div>
                                    <div class="sku">
                                        {{ product.skuValue }}
                                    </div>
                                </div>
                            </div>
                        </NuxtLink>
                        <div class="memberOrder_shop_price">
                            <div>
                                <!-- 定金预售：展示订单总额/应付（只展示一次，避免每个商品重复） -->
                                <template v-if="isDepositPresaleOrder(item) && pIndex === 0">
                                    <div style="display: flex; flex-direction: column; gap: 4px">
                                        <div style="display: flex; align-items: center; gap: 6px">
                                            <span style="color: #999">{{ $t("总额") }}</span>
                                            <FormatPrice
                                                v-model="item.totalAmount"
                                                :showText="false"
                                                :priceStyle="{ color: '#1a1a1a' }"
                                                :fontStyle="{ fontSize: '16px', fontWeight: 'bold', lineHeight: '16px' }"
                                                :currencyStyle="{ fontSize: '12px' }"
                                                :decimalsStyle="{ fontSize: '12px' }"
                                            ></FormatPrice>
                                        </div>
                                        <div style="display: flex; align-items: center; gap: 6px">
                                            <span style="color: #999">{{ $t("应付") }}</span>
                                            <FormatPrice
                                                v-model="item.unpaidAmount"
                                                :showText="false"
                                                :priceStyle="{ color: '#1a1a1a' }"
                                                :fontStyle="{ fontSize: '16px', fontWeight: 'bold', lineHeight: '16px' }"
                                                :currencyStyle="{ fontSize: '12px' }"
                                                :decimalsStyle="{ fontSize: '12px' }"
                                            ></FormatPrice>
                                        </div>
                                    </div>
                                </template>

                                <!-- 其他订单：保持原先商品单价展示 -->
                                <NuxtLink
                                    v-else-if="!isDepositPresaleOrder(item)"
                                    :title="product.productName"
                                    :to="urlFormat({ path: 'product', seoLink: product.seoLink, sn: product.productSn })"
                                    target="_blank"
                                >
                                    <FormatPrice
                                        v-model="product.price"
                                        :showText="false"
                                        :priceStyle="{ color: '#1a1a1a' }"
                                        :fontStyle="{ fontSize: '18px', fontWeight: 'bold', lineHeight: '18px' }"
                                        :currencyStyle="{ fontSize: '12px' }"
                                        :decimalsStyle="{ fontSize: '12px' }"
                                    ></FormatPrice>
                                </NuxtLink>
                            </div>
                            <NuxtLink
                                :title="product.productName"
                                :to="urlFormat({ path: 'product', seoLink: product.seoLink, sn: product.productSn })"
                                target="_blank"
                            >
                                <div style="color: #aaa">X{{ product.quantity }}</div>
                            </NuxtLink>
                        </div>
                    </div>
                    <div class="memberOrder_bot">
                        <div></div>
                        <div class="memberOrder_bot_but">
                            <div v-if="item.availableActions.confirmReceipt && !isCardSecretOrder(item)" style="margin-right: 8px">
                                <TigButton @click="viewLogistics(item)" class="deleteWeigh">
                                    {{ $t("查看物流") }}
                                </TigButton>
                            </div>
                            <div v-if="item.availableActions.toPay && canShowToPay(item)" style="margin-left: 8px">
                                <TigButton v-if="Number(item.offlinePaymentClaimStatus) === 2" theme type="primary" class="deleteWeigh" @click="openClaimEdit(item.orderId)">{{ $t("修改申请") }}</TigButton>
                                <NuxtLink v-else :to="'/order/pay/?id=' + item.orderId" target="_balnk">
                                    <TigButton theme type="primary" class="deleteWeigh">{{ $t("去支付") }}</TigButton>
                                </NuxtLink>
                            </div>
                            <div v-if="item.availableActions.offlinePaymentClaimDetail && Number(item.offlinePaymentClaimStatus) !== 2" style="margin-left: 8px">
                                <TigButton @click="openClaimDetail(item.orderId)" class="deleteWeigh">{{ $t("认款详情") }}</TigButton>
                            </div>
                            <div v-if="item.availableActions.cancelOrder" style="margin-left: 8px">
                                <TigButton @click="onCancelOrder(item.orderId)" class="deleteWeigh">
                                    {{ $t("取消订单") }}
                                </TigButton>
                            </div>
                            <div v-if="item.availableActions.delOrder">
                                <TigButton @click="onDelOrder(item.orderId)" class="deleteWeigh"> {{ $t("删除订单") }} </TigButton>
                            </div>
                            <div v-if="item.availableActions.confirmReceipt && item.orderType !== 9 && item.orderType !== 10" style="margin-left: 8px">
                                <TigButton theme type="primary" @click="onConfirmReceipt(item.orderId)" class="deleteWeigh">{{ $t("确认收货") }}</TigButton>
                            </div>
                            <div
                                v-if="
                                    item.availableActions.toAftersales &&
                                    item.orderSource !== 'pos' &&
                                    item.cycleCardInfo?.statusName !== '已过期' &&
                                    !isGiftCardAftersalesBlocked(item) &&
                                    !isOrderType14(item) &&
                                    !isCycleInfo(item) &&
                                    !isPickupCompleted(item)
                                "
                                style="margin-left: 8px"
                            >
                                <NuxtLink :to="'/member/return/info/?orderId=' + item.orderId">
                                    <TigButton class="deleteWeigh">{{ $t("申请售后") }}</TigButton>
                                </NuxtLink>
                            </div>
                            <div v-else-if="item.orderStatusName === '已确认，待发货' && isGiftCardAftersalesBlocked(item)">-</div>
                            <div
                                v-if="item.items && item.items[0]?.aftersalesItem && !isGiftCardAftersalesBlocked(item) && !isOrderType14(item) && !isCycleInfo(item)"
                                style="margin-left: 8px"
                            >
                                <NuxtLink :to="'/member/return/logInfo?id=' + item.items[0]?.aftersalesItem?.aftersaleId">
                                    <TigButton class="deleteWeigh">{{ $t("售后详情") }}</TigButton>
                                </NuxtLink>
                            </div>
                            <div v-if="item.availableActions.rebuy && !isSubCardOrder(item)" style="margin-left: 8px">
                                <TigButton theme type="primary" @click="onReBuy(item)" class="deleteWeigh">{{ $t("再次购买") }}</TigButton>
                            </div>
                            <!-- D1146：后端已在关闭时不下发 toComment，这里再加前端守卫，覆盖配置刚改完、前端 store 仍是旧值的窗口 -->
                            <div v-if="item.availableActions.toComment && commentEnabled" style="margin-left: 8px">
                                <NuxtLink :to="'/member/comment/info/?id=' + item.orderId">
                                    <TigButton class="deleteWeigh">{{ $t("去评价") }}</TigButton>
                                </NuxtLink>
                            </div>
                            <!--                                    拼团-->
                            <div v-if="item.joinStatus" style="margin-left: 8px">
                                <NuxtLink :to="'/member/group/info?id=' + item.grouponPromotionRecordId" target="_blank">
                                    <TigButton class="deleteWeigh">{{ $t("拼团详情") }}</TigButton>
                                </NuxtLink>
                            </div>
                        </div>
                    </div>
                </div>
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
            <el-dialog v-model="visibleSecurity" :title="$t('物流跟踪')" :footer="null" top="15vh" width="770px">
                <LogisticsTracking :trackingNo="teacking.trackingNo" :logisticsName="teacking.logisticsName" :id="teacking.orderId"></LogisticsTracking>
            </el-dialog>
            <OfflinePaymentClaimDetail v-model="claimDetailVisible" :order-id="claimDetailOrderId" :editable="claimDetailEditable" @submitted="loadFilter" />
        </div>
        <CancelOrderReasonDialog ref="cancelOrderReasonDialogRef" @success="onCancelSuccess" />
    </div>
</template>

<script lang="ts" setup>
import { computed, onMounted, reactive, ref } from "vue";
import type { OrderFilterParams, OrderFilterState } from "~/types/user/order.d";
import { delOrder, getOrderList, getOrderNum, reBuy, confirmReceipt } from "~/api/user/order";
import { getJoiningActivityDetail } from "~/api/joining/joining";
import { Pagination } from "~/components/list";
import { productSkuImage, urlFormat } from "~/utils/format";
import { isOverseas, isStore } from "~/utils/util";
import { message, Modal } from "ant-design-vue";
import { useRoute } from "vue-router";
import dayjs from "dayjs";
import LogisticsTracking from "~/components/member/order/LogisticsTracking.vue";
import OfflinePaymentClaimDetail from "~/components/member/order/OfflinePaymentClaimDetail.vue";
import { useCommonStore } from "~/store/common";
import CancelOrderReasonDialog from "~/components/member/order/CancelOrderReasonDialog.vue";
import { isGiftCardAftersalesBlocked } from "~/utils/giftCard";
import { canShowOrderPayment } from "~/utils/orderPayment";
const { t } = useI18n();
const commonStore = useCommonStore();
// D1146：商品评价总开关
const commentEnabled = computed(() => commonStore.productCommentEnabled === 1);
const props = defineProps({
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
const visibleSecurity = ref(false);
const claimDetailVisible = ref(false);
const claimDetailOrderId = ref(0);
const claimDetailEditable = ref(false);
interface menuType {
    type: string;
    value: string;
    num: number;
}
const teacking = ref<any>({});
const menuTypeList = reactive<menuType[]>([
    { type: "all", value: "全部", num: 0 },
    { type: "awaitPay", value: "待付款", num: 0 },
    { type: "awaitShipping", value: "待发货", num: 0 },
    { type: "awaitReceived", value: "待收货", num: 0 },
    { type: "awaitComment", value: "待评价", num: 0 }
]);
// D1146：评价关闭时隐藏「待评价」Tab。保留原数组以维持 loadNum 对 num 的写入，仅在渲染层过滤
const visibleMenuTypeList = computed(() => menuTypeList.filter((item) => item.type !== "awaitComment" || commentEnabled.value));
const changeType = (type: string) => {
    // URL 直接带 type=awaitComment（历史书签）时回落到全部，避免筛出一个不该存在的视图
    if (type === "awaitComment" && !commentEnabled.value) {
        type = "all";
    }
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
// 再次购买：跨境直接重购到结算；o2o/标准 拼团·次卡感知
const onReBuy = async (item: any) => {
    if (isOverseas()) {
        try {
            await reBuy({ id: item.orderId });
            message.success(t("正在跳转中，请稍后~"));
            setTimeout(function () {
                navigateTo(`/order/check`);
            }, 1500);
        } catch (error: any) {
            message.error(error.message);
        }
        return;
    }
    const orderId = item.orderId;
    const productId = item.items?.[0]?.productId;
    const joinStatus = item.joinStatus;
    try {
        if (joinStatus) {
            const result = await getJoiningActivityDetail({ productId });
            if (result.endTime > dayjs().format("YYYY-MM-DD HH:mm:ss")) {
                navigateTo(urlFormat({ path: "product", seoLink: result.seoLink, id: result.productId, sn: result.productSn }));
            } else {
                message.error(t("拼团已结束,可以按现价购买该商品"));
                await reBuy({ id: orderId });
                navigateTo(`/cart`);
            }
        } else if (isMainSubCardOrder(item)) {
            navigateTo(
                urlFormat({
                    path: "product",
                    seoLink: item.items[0]?.seoLink,
                    id: item.items[0]?.productId,
                    sn: item.items[0]?.productSn
                })
            );
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

/** 查看物流 */
const viewLogistics = (e: any) => {
    teacking.value = e;
    visibleSecurity.value = true;
};
const canShowToPay = canShowOrderPayment;
const openClaimDetail = (orderId: number) => {
    navigateTo(`/member/order/claimInfo?orderId=${Number(orderId)}`);
};
const openClaimEdit = (orderId: number) => {
    claimDetailOrderId.value = Number(orderId);
    claimDetailEditable.value = true;
    claimDetailVisible.value = true;
};

// 订单类型=14：不显示售后相关操作
const isOrderType14 = (order: any) => Number((order as any)?.orderType) === 14;
// 定金预售订单（saleMode=2）
const isDepositPresaleOrder = (order: any) => Number((order as any)?.presaleInfo?.saleMode ?? 0) === 2;

// —— o2o 融合守卫（数据驱动，跨境环境下相关字段为空自然不生效）——
// 次卡主订单
const isMainSubCardOrder = (item: any) => item.isCardOrder && Number(item.parentOrderSn) === 0;
// 次卡子订单
const isSubCardOrder = (item: any) => item.isCardOrder && Number(item.parentOrderSn) > 0;
// 周期购主订单
const isCycleInfo = (order: any) => order.isCycleOrder === 1;
// 自提订单已完成时不允许申请售后
const isPickupCompleted = (order: any) => order.isPickup && order.orderStatus === 5;

// 卡密商品无需物流，已发货状态不显示物流查看入口
const isCardSecretOrder = (order: OrderFilterState) => {
    const items = order.items ?? [];
    return items.length > 0 && items.every((item) => Number(item.productType) === 3);
};
</script>

<style lang="less" scoped>
@import "/assets/css/member/member";

// o2o Price 组件比跨境少一层 `.price-row { align-items: baseline }`，导致此处「整数16px+小数14px」在 .price.align-bottom(flex-end) 下小数视觉偏上。
// 只在本组件 scoped 内 :deep 到 .price 覆盖为 baseline 对齐（等价于跨境 .price-row 效果），不动 Price.vue 本体，避免影响其他页面。
:deep(.price) {
    align-items: baseline;
}

.order-list {
    .tag-and-input {
        display: flex;
        flex-direction: row;
        justify-content: space-between;

        .image {
            margin-bottom: 20px;
        }
    }
}
.orange {
    color: #ed6a0c;
}
.green {
    color: #31c19e;
}
.gray {
    color: #999999;
}
.memberOrder {
    margin-bottom: 16px;
    border: 1px solid #f0f1f2;
    border-radius: 4px;
    overflow: hidden;
    cursor: pointer;
    font-family: "Trebuchet MS", "Lucida Sans Unicode", "Lucida Grande", "Lucida Sans", Arial, sans-serif;
    .memberOrder_top {
        display: flex;
        align-items: center;
        justify-content: space-between;
        height: 48px;
        background-color: #f5f6fa;
        padding: 0 16px;
        border-bottom: 1px solid #f0f1f2;
        font-size: 14px;
        .memberOrder_top_left {
            display: flex;
            align-items: center;
            gap: 12px;
            font-size: 16px;
            font-weight: 700;
            line-height: 20px;
            color: #1a1a1a;
            .join-status {
                font-size: 13px;
                font-weight: 500;
            }
        }
        .icon {
            margin-top: 2px;
            margin-left: 3px;
        }
        &:hover {
            a {
                color: var(--general);
            }
            .icon {
                color: var(--general);
            }
        }
    }
    .memberOrder_middle {
        display: flex;
        align-items: center;
        justify-content: space-between;
        min-height: 48px;
        padding: 0 16px;
        border-bottom: 1px solid #f0f1f2;
        .memberOrder_middle_left {
            display: flex;
            align-items: center;
            gap: 24px;
            .order-shop {
                color: #666;
            }
        }
    }
    .memberOrder_shop {
        display: flex;
        padding: 12px 16px;
        .memberOrder_shop_text {
            margin: 0 16px;
            width: 680px;
            .text-ellipsis {
                font-size: 16px;
                color: #1a1a1a;
            }
            .sku {
                font-size: 14px;
                line-height: 18px;
                color: #888b94;
                margin-top: 4px;
            }
        }
        .memberOrder_shop_price {
            display: flex;
            justify-content: space-between;
            flex: 1;
        }
    }
    .memberOrder_bot {
        display: flex;
        align-items: center;
        justify-content: space-between;
        height: 48px;
        padding: 0 16px;
        border-top: 1px solid #f0f1f2;
        .memberOrder_bot_but {
            display: flex;
            align-items: center;
            font-weight: 500;
        }
    }
}
.flex_cc {
    display: flex;
    justify-content: center;
    align-items: center;
}
.fs_14 {
    font-size: 14px;
}
.activeert {
    color: #1a1a1a !important;
    font-weight: 700 !important;
}
.activeert::before {
    content: "";
    position: absolute;
    width: 36px;
    height: 3px;
    left: 50%;
    transform: translate(-50%);
    bottom: -1px;
    background: #1a1a1a;
}
.deleteWeigh {
    font-weight: 500 !important;
    border-radius: 4px;
}
// 拼团角标
.joining-tag {
    background: var(--general);
    border-radius: 2px;
    padding: 2px 5px;
    font-weight: bold;
    font-size: 10px;
    color: #ffffff;
    text-align: center;
    display: inline;
    margin-right: 5px;
}
.presale-text {
    color: var(--general);
    margin-right: 4px;
    font-weight: 500;
    line-height: 1;
    vertical-align: baseline;
}
</style>
