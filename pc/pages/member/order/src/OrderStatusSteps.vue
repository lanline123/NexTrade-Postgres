<!--
  订单详情 · 订单状态区
  - 预售订单：预售步骤
  - 物流跟踪入口
-->
<template>
    <div class="page-info-body" v-if="formState.isPresale || showLogisticsTracking">
        <div class="sub-title" v-if="formState.isPresale">{{ $t("预售状态") }}</div>
        <div class="presale-steps" v-if="formState.isPresale">
            <PresaleSteps
                :productPresale="formState?.presaleInfo?.productPresale"
                :presaleStatus="formState?.presaleInfo?.presaleStatus ?? 0"
                :total="formState"
                :deliveryType="presaleDeliveryType"
            />
        </div>
        <LogisticsTracking
            :trackingNo="formState.trackingNo"
            :logisticsName="formState.logisticsName"
            :shippingMethod="formState.shippingMethod"
            v-if="showLogisticsTracking"
            v-model:id="formState.orderId"
        ></LogisticsTracking>
    </div>
</template>

<script lang="ts" setup>
import { computed } from "vue";
import type { PropType } from "vue";
import type { OrderFormState } from "~/types/user/order.d";
import LogisticsTracking from "~/components/member/order/LogisticsTracking.vue";
import PresaleSteps from "~/pages/order/src/presaleSteps.vue";

const props = defineProps({
    formState: {
        type: Object as PropType<OrderFormState>,
        required: true
    }
});

// 是否次卡主订单
const isMainSubCardOrder = computed(() => {
    return props.formState.isCardOrder && Number(props.formState.parentOrderSn) === 0;
});

const showLogisticsTracking = computed(() => {
    return (
        !!props.formState.orderId &&
        (props.formState.orderStatus === 2 || props.formState.orderStatus === 5) &&
        props.formState.shippingMethod == 1 &&
        props.formState.orderSource !== "pos" &&
        ![9, 10].includes(props.formState.orderType as number) &&
        props.formState.isCycleOrder !== 1 &&
        !isMainSubCardOrder.value
    );
});

const presaleDeliveryType = computed<number | undefined>(() => {
    if (props.formState?.isPickup) return 2;
    const shippingTypeName = String(props.formState?.shippingTypeName || "");
    if (shippingTypeName.includes("同城")) return 3;
    return undefined;
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

.presale-steps {
    margin-top: 20px;
}
</style>
