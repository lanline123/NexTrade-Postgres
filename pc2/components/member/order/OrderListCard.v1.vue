<!--
  订单列表 · 单个订单卡片
  - 展示单条订单：订单号、下单门店（o2o）、下单时间、商品列、金额、交易状态、操作
  - 数据由父组件通过 :item 下发；操作通过 emit 回父组件执行
-->
<template>
    <table class="order-table">
        <thead>
            <tr>
                <th colspan="4" class="order-table-title">
                    {{ $t("订单号") }}：<NuxtLink :to="'/member/order/info?id=' + item.orderId" target="_blank">
                        <span class="font-color">{{ item.orderSn }}</span>
                    </NuxtLink>
                    <!-- 门店 start：下单门店链接（o2o，融合后此处整体切割） -->
                    <span v-if="item.shop && item.shop.shopTitle" style="margin-left: 30px"
                        >{{ $t("下单门店") }}：<NuxtLink :to="{ path: '/store/info', query: { shopId: item.shopId } }" target="_blank">
                            <span class="font-color">{{ item.shop.shopTitle }}</span>
                        </NuxtLink></span
                    >
                    <!-- 门店 end -->
                    <span style="margin-left: 30px">{{ $t("下单时间") }}：{{ item.addTime }}</span>
                </th>
            </tr>
        </thead>
        <tbody>
            <tr class="order-tr">
                <td class="column-1">
                    <div v-for="product in item.items" :key="product.productId">
                        <NuxtLink
                            :title="product.productName"
                            :to="urlFormat({ path: 'product', id: product.productId, sn: product.productSn, shopId: item.shopId > 0 ? item.shopId : undefined })"
                            class="nuxt"
                            target="_blank"
                        >
                            <el-image
                                :src="productSkuImage(product)"
                                loading="lazy"
                                style="height: 40px; width: 40px; transition: opacity 0.2s linear; flex-shrink: 0"
                                :alt="product.productName || ''"
                            />
                            <div>
                                <div class="text-ellipsis product-name-wrapper">
                                    <span v-if="item.isPresale" class="presale-text">{{ $t("预售") }}</span>
                                    <span v-if="item.isPresale">| </span>
                                    <!-- 拼团 -->
                                    <span v-if="item.joinStatus" class="joining-tag">拼团</span>
                                    <span class="product-name"> {{ product.productName }}</span>
                                </div>
                            </div>
                        </NuxtLink>
                        <div style="color: #aaa">X{{ product.quantity }}</div>
                        <div></div>
                    </div>
                </td>
                <td class="column-2-4">
                    <div>
                        <div class="flex">
                            <div>{{ $t("总额") }}:</div>
                            <FormatPrice v-model="item.totalAmount" :showText="false"></FormatPrice>
                        </div>
                        <template v-if="item.orderStatus == 0 && item.unpaidAmount > 0">
                            <div class="black">{{ $t("应付") }}</div>
                            <div class="money">
                                <FormatPrice v-model="item.unpaidAmount" :showText="false"></FormatPrice>
                            </div>
                        </template>
                        <div>{{ item.payTypeId === 1 ? $t("在线支付") : item.payTypeId === 2 ? $t("货到付款") : $t("线下支付") }}</div>
                    </div>
                </td>
                <td class="column-2-4">
                    <div>
                        <div
                            :class="{
                                orange: item.orderStatus === 0,
                                black: item.orderStatus === 1 || item.orderStatus === 2 || item.orderStatus === 4,
                                gray: item.orderStatus === 3,
                                green: item.orderStatus === 5
                            }"
                        >
                            {{ $t(item.orderStatusName) }}
                        </div>
                        <div v-if="item.cancelRemark?.trim()" class="gray">
                            {{ $t("取消原因") }}：{{ item.cancelRemark }}
                        </div>
                        <div
                            v-if="item.joinStatusDesc"
                            :class="{
                                orange: item.joinStatus === 1,
                                green: item.joinStatus === 2,
                                gray: item.joinStatus === 3
                            }"
                            style="text-align: center"
                        >
                            {{ $t(item.joinStatusDesc) }}
                        </div>
                        <div class="black">
                            <NuxtLink :to="'/member/order/info?id=' + item.orderId" target="_blank">{{ $t("订单详情") }}</NuxtLink>
                        </div>
                        <div
                            v-if="
                                (item.orderStatus == 2 || item.orderStatus == 5) &&
                                item.shippingMethod == 1 &&
                                ![9, 10].includes(item.orderType) &&
                                item.isCycleOrder !== 1 &&
                                !isMainSubCardOrder(item)
                            "
                            class="black"
                        >
                            <el-popover :width="600" placement="bottom" @show="showPopover = true" @hide="showPopover = false">
                                <template v-if="showPopover">
                                    <LogisticsTracking :trackingNo="item.trackingNo" :logisticsName="item.logisticsName" :id="item.orderId">
                                    </LogisticsTracking>
                                </template>
                                <template #reference>
                                    <span style="cursor: pointer">
                                        <i> {{ $t("物流跟踪") }}</i>
                                        <i style="font-size: 12px" class="iconfont-pc icon-xiajiantou1"></i>
                                    </span>
                                </template>
                            </el-popover>
                        </div>
                    </div>
                </td>
                <td class="column-2-4">
                    <div>
                        <div v-if="item.availableActions.toPay">
                            <NuxtLink :to="'/order/pay/?id=' + item.orderId" target="_balnk">
                                <el-button size="small" type="primary">{{ $t("付款") }}</el-button>
                            </NuxtLink>
                        </div>
                        <a v-if="item.availableActions.cancelOrder" @click="emit('cancel-order', item.orderId)"> {{ $t("取消订单") }} </a>
                        <a v-if="item.availableActions.delOrder" @click="emit('del-order', item.orderId)"> {{ $t("删除") }} </a>
                        <div v-if="item.availableActions.confirmReceipt && item.orderType !== 9 && item.orderType !== 10" class="mb10">
                            <el-button type="primary" size="small" @click="emit('confirm-receipt', item.orderId)">{{ $t("确认收货") }}</el-button>
                        </div>
                        <div
                            v-if="
                                item.availableActions.toAftersales &&
                                item.orderSource !== 'pos' &&
                                item.cycleCardInfo?.statusName !== '已过期' &&
                                !isGiftCardAftersalesBlocked(item) &&
                                !isCycleInfo(item) &&
                                !isPickupCompleted(item)
                            "
                            class="mb10"
                        >
                            <NuxtLink :to="'/member/return/info/?orderId=' + item.orderId">
                                <el-button size="small">{{ $t("申请售后") }}</el-button>
                            </NuxtLink>
                        </div>
                        <div v-if="item.items && item.items[0]?.aftersalesItem && !isGiftCardAftersalesBlocked(item) && !isCycleInfo(item)" class="mb10">
                            <NuxtLink :to="'/member/return/logInfo?id=' + item.items[0]?.aftersalesItem?.aftersaleId">
                                <el-button size="small">{{ $t("售后详情") }}</el-button>
                            </NuxtLink>
                        </div>
                        <div v-if="item.availableActions.rebuy && !isSubCardOrder(item)" class="mb10">
                            <el-button size="small" @click="emit('re-buy', item)">{{ $t("再次购买") }}</el-button>
                        </div>

                        <div v-if="item.availableActions.toComment" class="mb10">
                            <NuxtLink :to="'/member/comment/info/?id=' + item.orderId">
                                <el-button size="small">{{ $t("去评价") }}</el-button>
                            </NuxtLink>
                        </div>
                        <!-- 拼团 -->
                        <div v-if="item.joinStatus">
                            <NuxtLink :to="'/member/group/info?id=' + item.grouponPromotionRecordId" target="_blank">
                                <el-button size="small" type="primary">{{ $t("拼团详情") }}</el-button>
                            </NuxtLink>
                        </div>
                    </div>
                </td>
            </tr>
        </tbody>
    </table>
</template>

<script lang="ts" setup>
import { ref } from "vue";
import type { PropType } from "vue";
import type { OrderFilterState } from "~/types/user/order.d";
import { productSkuImage, urlFormat } from "~/utils/format";
import LogisticsTracking from "~/components/member/order/LogisticsTracking.vue";
import { isGiftCardAftersalesBlocked } from "~/utils/giftCard";

defineProps({
    item: {
        type: Object as PropType<OrderFilterState>,
        required: true
    }
});

const emit = defineEmits(["cancel-order", "del-order", "confirm-receipt", "re-buy"]);

// 单卡片自身的物流弹窗展开状态
const showPopover = ref<boolean>(false);

// 是否次卡主订单
const isMainSubCardOrder = (item: any) => {
    return item.isCardOrder && Number(item.parentOrderSn) === 0;
};
// 是否次卡子订单
const isSubCardOrder = (item: any) => {
    return item.isCardOrder && Number(item.parentOrderSn) > 0;
};
// 自提订单已完成时不允许申请售后
const isPickupCompleted = (order: any) => {
    return order.isPickup && order.orderStatus === 5;
};
// 是否周期购主订单
const isCycleInfo = (order: any) => {
    return order.isCycleOrder === 1;
};
</script>

<style lang="less" scoped>
@import "/assets/css/member/member.v1";

.order-table {
    color: #666;
    border: 1px solid #eee;
    width: 100%;
    background-color: #f7f7f7;
    height: 40px;
    line-height: 40px;
    margin-bottom: 10px;

    & > thead > tr > th {
        padding-left: 10px;
    }

    .order-tr {
        background-color: white;
        box-sizing: border-box;

        .column-1 {
            width: 64%;
            border: 1px solid #eee;

            .nuxt {
                display: flex;
                gap: 10px;
                flex: 1;
            }

            & > div {
                padding: 15px;
                box-sizing: border-box;
                display: flex;
                align-items: center;
                flex-direction: row;
                border-bottom: 1px solid #eee;
                width: 100%;

                & > span {
                    flex: 1;
                    margin-left: 10px;
                    line-height: 2;
                }

                & > div {
                    margin-left: 10px;
                    width: 60px;
                    text-align: center;
                }
            }

            & > div:last-child {
                border-bottom: none;
            }
        }

        .column-2-4 {
            width: 12%;
            border: 1px solid #eee;

            & > div {
                display: flex;
                height: 100%;
                padding: 20px 0;
                box-sizing: border-box;
                flex-direction: column;
                justify-content: flex-start;
                align-items: center;
                color: #aaaaaa;

                & > .black {
                    color: black;
                }

                & > .money {
                    text-align: center;
                    width: 80%;
                    border-bottom: 1px solid #e5e5e5;
                }

                & > div {
                    line-height: 2;
                }
            }

            a {
                cursor: pointer;
            }
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

// 拼团
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

.product-name-wrapper {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    display: block;
    width: 370px;
}

.presale-text {
    color: var(--general);
    margin-right: 4px;
    font-weight: 500;
    line-height: 1;
    vertical-align: baseline;
}

.product-name {
    vertical-align: baseline;
    display: inline;
}
</style>
