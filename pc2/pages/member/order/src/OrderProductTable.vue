<!--
  订单详情 · 订单商品区
  - 商品明细表（图片/规格/价格/数量/小计/售后操作）
  - 金额合计（商品总价/附加费/运费/余额/积分/优惠/实付款）
  - 礼品卡兑换/转赠操作区
-->
<template>
    <div class="page-info-body">
        <div class="sub-title">{{ $t("订单商品") }}</div>
        <table class="order-table-info">
            <thead>
                <tr class="receipt-header">
                    <th>{{ $t("商品名称") }}</th>
                    <th>{{ $t("商品价格") }}</th>
                    <th>{{ $t("购买数量") }}</th>
                    <th>{{ $t("小计") }}</th>
                    <th
                        v-if="
                            formState.availableActions.toAftersales &&
                            !giftCardAftersalesBlocked &&
                            !isGiftCardExchangeOrder &&
                            formState.orderType !== 14 &&
                            formState.isCycleOrder !== 1
                        "
                    >
                        {{ $t("操作") }}
                    </th>
                </tr>
            </thead>
            <tbody>
                <tr v-for="item in formState.items" class="receipt-body">
                    <td class="receipt-item">
                        <div class="product-cell">
                            <img :alt="item.productName" :src="productSkuImage(item)" class="img-border" />
                            <div class="product-cell__info">
                                <div class="product-cell__name">{{ item.productName }}</div>
                                <div class="sku-style" v-if="item.skuData">
                                    <div v-for="sku in item.skuData">{{ sku.name }}：{{ sku.value }}</div>
                                </div>
                                <template v-if="item.extraSkuData && item.extraSkuData.length">
                                    <div class="extraskudata">
                                        <template v-for="attr in item.extraSkuData" :key="attr">
                                            <div class="extraskudata-item">
                                                <div class="title">{{ attr.attrName }}</div>
                                                <div class="line-box">
                                                    <div class="line"></div>
                                                </div>
                                                <div class="content">
                                                    <span class="content-text">{{ attr.attrValue }}</span>
                                                    <FormatPrice class="price" v-model="attr.attrPrice"> </FormatPrice>
                                                    <span class="num">x{{ item.quantity }}</span>
                                                </div>
                                            </div>
                                        </template>
                                    </div>
                                </template>
                            </div>
                        </div>
                    </td>
                    <td class="receipt-item price">
                        <FormatPrice v-model="item.price"></FormatPrice>
                    </td>
                    <td class="receipt-item price">{{ item.quantity }}</td>
                    <td class="receipt-item price">
                        <FormatPrice v-model="item.subtotal"></FormatPrice>
                    </td>
                    <template v-if="formState.payStatus === 2 && !giftCardAftersalesBlocked && !isGiftCardExchangeOrder && formState.isCycleOrder !== 1">
                        <td class="receipt-item operation">
                            <template v-if="item.isGift === 0">
                                <template
                                    v-if="
                                        formState.availableActions.toAftersales &&
                                        formState.orderSource !== 'pos' &&
                                        formState.orderType !== 14 &&
                                        formState.isCycleOrder !== 1 &&
                                        !isPickupCompleted &&
                                        formState.cycleCardInfo?.statusName !== '已过期' &&
                                        canGiftCardItemApplyAftersales(formState, item) &&
                                        item.quantity > (item.aftersalesNumber || 0)
                                    "
                                >
                                    <NuxtLink target="_blank" :to="'/member/return/Info?orderId=' + formState.orderId + '&itemId=' + item.itemId">
                                        <el-button size="small">{{ $t("申请售后") }}</el-button>
                                    </NuxtLink>
                                </template>
                                <template v-else-if="item.aftersalesItem">
                                    <NuxtLink target="_blank" :to="'/member/return/logInfo?id=' + item?.aftersalesItem?.aftersaleId">
                                        <el-button size="small">{{ $t("售后详情") }}</el-button>
                                    </NuxtLink>
                                </template>
                            </template>
                        </td>
                    </template>
                </tr>
            </tbody>
            <tfoot>
                <tr class="receipt-footer">
                    <th
                        :colspan="
                            formState.availableActions.toAftersales && !giftCardAftersalesBlocked && !isGiftCardExchangeOrder && formState.orderType !== 14
                                ? 3
                                : 2
                        "
                    ></th>
                    <th class="receipt-footer-it one" colspan="2">
                        <div class="item">
                            <div class="label">{{ $t("商品总价") }}：</div>
                            <div class="text">
                                <FormatPrice v-model="formState.productAmount"></FormatPrice>
                            </div>
                        </div>
                        <div class="item" v-if="formState.serviceFee && formState.serviceFee > 0">
                            <div class="label">{{ $t("附加费用") }}：</div>
                            <div class="text">
                                <FormatPrice v-model="formState.serviceFee"></FormatPrice>
                            </div>
                        </div>
                        <div class="item" v-if="showShippingFeeRow">
                            <div class="label">{{ $t(shippingFeeLabel) }}：</div>
                            <div class="text">
                                <template v-if="Number(formState.shippingFee || 0) > 0">
                                    <FormatPrice v-model="formState.shippingFee"></FormatPrice>
                                </template>
                                <template v-else>
                                    {{ $t(shippingFeeFreeText) }}
                                </template>
                            </div>
                        </div>
                        <div class="item" v-if="formState.balance! > 0">
                            <div class="label">{{ $t("使用余额") }}：</div>
                            <div class="text">
                                <FormatPrice v-model="formState.balance"></FormatPrice>
                            </div>
                        </div>
                        <div class="item" v-if="formState.pointsAmount! > 0">
                            <div class="label">{{ $t("{0}减免", [commonStore.integralName]) }}：</div>
                            <div class="text">
                                <FormatPrice v-model="formState.pointsAmount"></FormatPrice>
                            </div>
                        </div>
                        <div class="item" v-if="formState.discountAmount! > 0">
                            <div class="label">{{ $t("其他优惠") }}：</div>
                            <div class="text">
                                -
                                <FormatPrice v-model="formState.discountAmount"></FormatPrice>
                            </div>
                        </div>
                        <div class="item" v-if="formState.couponAmount! > 0">
                            <div class="label">{{ $t("优惠券优惠") }}：</div>
                            <div class="text">
                                -
                                <FormatPrice v-model="formState.couponAmount"></FormatPrice>
                            </div>
                        </div>
                        <div class="item" v-if="formState.unpaidAmount && formState.unpaidAmount > 0">
                            <div class="label">{{ $t("待支付金额") }}：</div>
                            <div class="text">
                                <FormatPrice v-model="formState.unpaidAmount"></FormatPrice>
                            </div>
                        </div>
                        <div class="item">
                            <span v-if="formState.payStatus == 2">{{ $t("实付款") }}：</span>
                            <span v-else>{{ $t("应付款") }}：</span>
                            <FormatPrice v-model="formState.totalAmount" :fontStyle="{ fontSize: '20px', color: 'var(--price)' }"> </FormatPrice>
                        </div>
                    </th>
                </tr>
            </tfoot>
        </table>
        <template v-if="isGiftCardProduct && showGiftCardExchange">
            <giftCardActions
                :cards="unUseGiftCards"
                :show-transfer="unUseGiftCards ? unUseGiftCards.length > 0 : false"
                :default-card-number="copiedCardNumber"
                :default-card-password="copiedCardPassword"
            />
        </template>
    </div>
</template>

<script lang="ts" setup>
import { computed } from "vue";
import type { PropType } from "vue";
import type { OrderFormState } from "~/types/user/order.d";
import type { GiftCardInfo } from "~/types/giftCard/giftCard";
import giftCardActions from "~/pages/giftCard/src/giftCardActions.vue";
import { useCommonStore } from "~/store/common";
import { productSkuImage } from "~/utils/format";
import { canGiftCardItemApplyAftersales, isGiftCardAftersalesBlocked } from "~/utils/giftCard";
const props = defineProps({
    formState: {
        type: Object as PropType<OrderFormState>,
        required: true
    },
    copiedCardNumber: {
        type: String,
        default: ""
    },
    copiedCardPassword: {
        type: String,
        default: ""
    }
});

const commonStore = useCommonStore();

/** 判断是否为礼品卡商品（实体卡type=7/电子卡type=8） */
const isGiftCardProduct = computed(() => {
    return props.formState.items?.some((item) => item.productType === 7 || item.productType === 8);
});
const giftCardAftersalesBlocked = computed(() => isGiftCardAftersalesBlocked(props.formState));
/** 判断是否为礼品卡兑换订单（orderType=12） */
const isGiftCardExchangeOrder = computed(() => {
    return props.formState.orderType === 12;
});
// 自提订单已完成时不允许申请售后
const isPickupCompleted = computed(() => {
    return props.formState.isPickup && props.formState.orderStatus === 5;
});

/**
 * @description 判断是否显示礼品卡兑换/转赠操作区
 * 实体卡不显示，订单未支付或已取消也不显示
 */
const showGiftCardExchange = computed(() => {
    // 是实体卡就不显示兑换按钮
    if (props.formState.items?.some((item) => item.productType === 7)) {
        return false;
    }
    return !(props.formState.orderStatus == 0 || props.formState.orderStatus == 3);
});
/** 过滤出未使用的礼品卡（status=0） */
const unUseGiftCards = computed(() => {
    return props.formState.giftCardOrder?.giftCardOrderInfo.filter((item: GiftCardInfo) => item.status == 0);
});

const deliveryOrderInfo = computed(() => props.formState?.deliveryOrderInfo);
const isDeliveryOrder = computed(() => deliveryOrderInfo.value?.type === 1);

/**
 * - 自提：不显示
 * - 预约（orderType=9/10）：上门服务费；0元显示“免费上门”
 * - 同城配送（有配送员信息 / 配送方式含“同城”）：配送费；0元显示“免配送费”
 * - 其它：快递；0元显示“包邮”
 */
const isPickupOrder = computed(() => {
    const shippingTypeName = String(props.formState?.shippingTypeName || "");
    // 优先用后端返回的配送方式名称判断
    if (shippingTypeName.includes("自提")) return true;
    // 兜底：存在自提信息且不是预约上门时，视为自提订单
    return !!props.formState?.isPickup && !props.formState?.isReservationHome && !!props.formState?.pickupDetail;
});
const isReserveOrder = computed(() => props.formState?.orderType === 10);
const isLocalDeliveryOrder = computed(() => {
    const shippingTypeName = String(props.formState?.shippingTypeName || "");
    return isDeliveryOrder.value || shippingTypeName.includes("同城");
});
const isNoDeliveryOrder = computed(() => {
    // 优先按商品类型判断：虚拟商品=2，卡密商品=3，付费内容=4，预约到店=9，电子礼品卡=8（这些都无需配送，不展示运费）
    const noDeliveryProductTypes = new Set([2, 3, 4, 9, 8]);
    const items = (props.formState?.items || []) as any[];
    const hasNoDeliveryProductType = items.some((item) => noDeliveryProductTypes.has(Number(item?.productType)));
    if (hasNoDeliveryProductType) return true;

    // 兜底：配送方式名称为“无需配送”
    const shippingTypeName = String(props.formState?.shippingTypeName || "");
    return shippingTypeName.includes("无需配送");
});
const showShippingFeeRow = computed(() => {
    // 自提：不显示
    if (isPickupOrder.value) return false;
    // 预约/同城：始终显示（0元显示对应文案）
    if (isReserveOrder.value || isLocalDeliveryOrder.value) return true;
    // 其它订单：若无需配送则不显示
    if (isNoDeliveryOrder.value) return false;
    // 默认（快递等）：显示（0元显示“包邮”）
    return true;
});
const shippingFeeLabel = computed(() => {
    if (isReserveOrder.value) return "上门服务费";
    if (isLocalDeliveryOrder.value) return "配送费";
    return "快递";
});
const shippingFeeFreeText = computed(() => {
    if (isReserveOrder.value) return "免费上门";
    if (isLocalDeliveryOrder.value) return "免配送费";
    return "包邮";
});
</script>

<style lang="less" scoped>
@import "/assets/css/member/member";

.sub-title {
    color: #333333;
    font-size: 16px;
    font-weight: 400;
    letter-spacing: 0;
    display: inline-block;
}

.extraskudata {
    color: #999;

    .extraskudata-item {
        display: flex;

        .title {
            padding-right: 10px;
        }

        .content-text {
            padding-right: 5px;
        }

        .content {
            .price {
                width: auto;
            }
        }

        .num {
            padding-left: 5px;
        }
    }
}

.order-table-info {
    width: 100%;
    margin: 10px auto;
    background-color: #fcfcfc;
    border-collapse: collapse;
    border: 1px solid #eee;

    .receipt-header {
        line-height: 2;
        border-bottom: 1px solid #eeeeee;

        & > th {
            padding: 10px;
            white-space: nowrap;
        }
    }

    .receipt-body {
        background-color: white;
        border: 1px solid #eeeeee;

        .receipt-item {
            padding: 10px;
            border: 1px solid #eeeeee;
        }

        .sku-style {
            width: 100%;
            display: flex;
            flex-direction: column;

            & > div {
                width: 100%;
            }
        }

        & > td:first-child > div {
            & > img {
                height: 60px;
                width: 60px;
            }

            display: flex;
            align-items: flex-start;
            gap: 20px;
            flex-wrap: nowrap;
        }

        /* 商品名称过长时，允许在右侧区域内换行，而不是把整块挤到下一行 */
        .product-cell__info {
            flex: 1;
            min-width: 0;
            overflow-wrap: anywhere;
        }

        /* 商品名称最多两行，超出显示省略号 */
        .product-cell__name {
            display: -webkit-box;
            -webkit-box-orient: vertical;
            -webkit-line-clamp: 2;
            line-clamp: 2;
            overflow: hidden;
            text-overflow: ellipsis;
            word-break: break-word;
            line-height: 1.4;
            max-height: calc(1.4em * 2);
        }

        .operation {
            width: 80px;
        }
    }

    .receipt-footer {
        border-bottom: 1px dashed #e8e8e8;

        .receipt-footer-it {
            text-align: right;
            padding: 10px;
            line-height: 2;

            .item {
                display: flex;
                justify-content: space-between;
                align-items: center;
                line-height: 28px;
            }

            .two {
                color: #888;
                font-size: 14px;
            }
        }
    }
}
</style>
