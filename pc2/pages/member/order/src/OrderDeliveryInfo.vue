<!--
  订单详情 · 订单信息区
  - 自提订单信息（o2o）
  - 预约订单信息 + 服务人员（o2o）
  - 普通订单：收货信息 / 配送方式 / 配送员（o2o）/ 支付方式 / 虚拟内容
-->
<template>
    <div class="page-info-body">
        <div class="sub-title">{{ $t("订单信息") }}</div>

        <!-- 门店 start：自提订单信息 isReservationHome 是否预约上门（o2o） -->
        <template
            v-if="
                (formState?.isPickup &&
                    !formState.isReservationHome &&
                    formState.orderStatus === 2 &&
                    formState.orderType !== 10 &&
                    formState.orderType !== 9) ||
                (formState.orderType === 2 && formState.pickupDetail && formState?.isPickup)
            "
        >
        </template>
        <!-- 预约订单信息 拼团并且不是自提formState.orderType === 2 && !formState.pickupDetail（o2o） -->
        <template
            v-else-if="
                formState.orderType === 10 ||
                formState.orderType === 9 ||
                formState.orderReservation?.orderId ||
                (formState.orderType === 2 && !formState.pickupDetail && formState?.isPickup)
            "
        >
            <ul class="order-ul" style="margin-top: 20px" v-if="showServiceStaffInfo">
                <li v-if="deliveryStaffName">
                    {{ $t("服务人员姓名") }}：<span>{{ deliveryStaffName }}</span>
                </li>
                <li v-if="deliveryStaffPhone">
                    {{ $t("联系电话") }}：<span>{{ deliveryStaffPhone }}</span>
                </li>
            </ul>
        </template>
        <!-- 门店 end -->
        <div v-else>
            <ul class="order-ul" style="margin-top: 20px">
                <template v-if="(formState.userAddress || formState.consignee || formState.mobile) && formState.orderSource !== 'pos'">
                    <li>
                        {{ $t("收货信息") }}：<span>{{ formState.userAddress }} {{ formState.consignee }} {{ formState.mobile }}</span>
                    </li>
                </template>
                <template v-if="formState.shippingTypeName">
                    <li>
                        {{ $t("配送方式") }}：<span>{{ $t(formState.shippingTypeName) }}</span>
                    </li>
                    <!-- 门店 start：配送员信息（o2o） -->
                    <template v-if="showDeliveryStaffInfo">
                        <li v-if="deliveryStaffName">
                            {{ $t("配送员") }}：<span>{{ deliveryStaffName }}</span>
                        </li>
                        <li v-if="deliveryStaffPhone">
                            {{ $t("联系电话") }}：<span>{{ deliveryStaffPhone }}</span>
                        </li>
                    </template>
                    <!-- 门店 end -->
                </template>
                <li>
                    {{ $t("支付方式") }}：
                    <span v-if="isGiftCardExchangeOrder">{{ $t("礼品卡兑换") }}</span>
                    <span v-else>{{
                        formState.payTypeId === 1 ? $t("在线支付") : formState.payTypeId === 2 ? $t("货到付款") : $t("线下支付")
                    }}</span>
                </li>
                <template v-if="virtualSampleItem">
                    <li class="virtualsample">
                        {{ $t("虚拟内容") }}：<span class="virtualsample-value" v-html="virtualSampleItem.virtualSample"></span>
                    </li>
                </template>
            </ul>
        </div>
    </div>
</template>

<script lang="ts" setup>
import { computed } from "vue";
import type { PropType } from "vue";
import type { OrderFormState } from "~/types/user/order.d";

const props = defineProps({
    formState: {
        type: Object as PropType<OrderFormState>,
        required: true
    }
});

/** 判断是否为礼品卡兑换订单（orderType=12） */
const isGiftCardExchangeOrder = computed(() => {
    return props.formState.orderType === 12;
});

// 虚拟内容：按商品判断（商品是虚拟商品 productType === 2 且有 virtualSample 就显示），不再依赖订单类型
const virtualSampleItem = computed(() => {
    const items = ((props.formState as any)?.items || []) as any[];
    return items.find((item) => Number(item?.productType) === 2 && !!item?.virtualSample) || null;
});

const deliveryOrderInfo = computed(() => props.formState?.deliveryOrderInfo);
const isDeliveryOrder = computed(() => deliveryOrderInfo.value?.type === 1);
const deliveryStaffName = computed(() => deliveryOrderInfo.value?.staffName || "");
const deliveryStaffPhone = computed(() => deliveryOrderInfo.value?.staffPhone || "");
const showDeliveryStaffInfo = computed(() => isDeliveryOrder.value && (!!deliveryStaffName.value || !!deliveryStaffPhone.value));
const showServiceStaffInfo = computed(() => !!deliveryStaffName.value || !!deliveryStaffPhone.value);
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

.order-ul {
    li {
        line-height: 3;
        color: #666;

        span {
            color: #333333;
        }
    }
}

.virtualsample {
    display: flex;
    width: 100%;

    .virtualsample-value {
        flex: 1;
        overflow: hidden;

        img {
            display: block;
            width: 100%;
            height: auto;
        }
    }
}
</style>
