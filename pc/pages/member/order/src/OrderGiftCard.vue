<!--
  订单详情 · 礼品卡卡片信息区（电子礼品卡且已生成卡信息时展示）
-->
<template>
    <div class="page-info-body">
        <div class="flex align-end">
            <div class="sub-title">{{ $t("卡片信息") }}</div>
            <div class="gift-card-num">（{{ $t("共") }}{{ formState.giftCardOrder?.num }}{{ $t("张") }}）</div>
        </div>
        <ul class="gift-card-list">
            <li v-for="item in formState.giftCardOrder?.giftCardOrderInfo" :key="item.giftCardsId" class="gift-card-item">
                <giftCard :card-info="item" card-size="large" @copy-all="onCopyAll" />
            </li>
        </ul>
    </div>
</template>

<script lang="ts" setup>
import type { PropType } from "vue";
import type { OrderFormState } from "~/types/user/order.d";
import giftCard from "~/pages/giftCard/src/giftCard.vue";

defineProps({
    formState: {
        type: Object as PropType<OrderFormState>,
        required: true
    }
});

const emit = defineEmits(["copy-all"]);

const onCopyAll = (cardNumber: string, cardPassword: string) => emit("copy-all", cardNumber, cardPassword);
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

.gift-card-num {
    color: #a4adb3;
    font-size: 12px;
}
.gift-card-list {
    padding: 16px 0;
    display: flex;
    gap: 30px;
    column-gap: 40px;
    flex-wrap: wrap;
    .gift-card-item {
        width: 420px;
        height: 232px;
    }
}
</style>
