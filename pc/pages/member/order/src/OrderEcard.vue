<!--
  订单详情 · 卡券信息区（商品含电子卡券时展示卡号/卡密）
-->
<template>
    <div class="page-info-body">
        <div class="sub-title">{{ $t("卡券信息") }}</div>
        <template v-for="item in formState.items">
            <div class="sub-sub-title">{{ item.productName }}</div>
            <ul class="order-ul virtualsample-box" v-if="item.eCard.length > 0">
                <li class="virtualsample-item" v-for="subItem in item.eCard" :key="subItem">
                    <div class="item">
                        {{ $t("卡号") }}：<span>{{ subItem.cardNumber }}</span>
                    </div>
                    <div class="item">
                        {{ $t("卡密") }}：<span>{{ subItem.cardPwd }}</span>
                    </div>
                </li>
            </ul>
        </template>
    </div>
</template>

<script lang="ts" setup>
import type { PropType } from "vue";
import type { OrderFormState } from "~/types/user/order.d";

defineProps({
    formState: {
        type: Object as PropType<OrderFormState>,
        required: true
    }
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

.sub-sub-title {
    font-size: 14px;
    padding-top: 20px;
    color: #333;
}

.order-ul {
    li {
        line-height: 3;
        color: #666;

        span {
            color: #333333;
        }
    }

    &.virtualsample-box {
        .virtualsample-item {
            display: flex;

            .item {
                padding-right: 10px;

                &:last-child {
                    padding-right: 0;
                }
            }
        }
    }
}
</style>
