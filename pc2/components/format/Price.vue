<template>
    <div class="price align-bottom" :style="priceStyle">
        <template v-if="showB2bText">
            <div class="text">{{ $t(b2bText) }}</div>
        </template>
        <template v-else>
            <div v-if="props.currencyFormat"
                :class="{ 'align-top': currencyAlignment === 'top', 'align-bottom': currencyAlignment === 'bottom' }"
                :style="hasContent(currencyStyle) ? currencyStyle : fontStyle" class="util">
                {{ currency }}
            </div>
            <div class="num" :style="fontStyle">{{ price.integer }}</div>
            <template v-if="price.decimals">
                <div class="num decimal" :style="hasContent(decimalsStyle) ? decimalsStyle : fontStyle">{{
                    price.decimals }}</div>
            </template>
        </template>
    </div>
</template>
<script lang="ts" setup>
import { computed } from "vue";
import { useCommonStore } from "~/store/common";
import { useUserStore } from "~/store/user";
import { isOverseas, moneyFormat, isB2B } from "@/utils/util";
import { splitPriceParts } from "@/utils/priceFormat";

const commonStore = useCommonStore();

const userStore = useUserStore();

const props = defineProps({
    modelValue: [String, Number],
    currencyAlignment: { type: String, default: "bottom" },
    currencyFormat: { type: Boolean, default: true },
    currencyStyle: {
        type: Object,
        default: {}
    },
    fontStyle: {
        type: Object,
        default: {}
    },
    decimalsStyle: {
        type: Object,
        default: {}
    },
    showText: {
        type: Boolean,
        default: true
    },
    completeDecimal: {
        type: Boolean,
        default: true
    },
    // 根节点自定义样式
    priceStyle: {
        type: Object,
        default: () => ({})
    }
});
const hasContent = (styleObj: any) => {
    return styleObj && Object.keys(styleObj).length > 0;
};

const b2bText = computed(() => {
    let str = "登录查看";
    if (userStore.token) {
        str = "实名查看";
    }

    return str;
});
const showB2bText = computed(() => {
    let value = false;
    if (isB2B() && props.showText && commonStore.isIdentity === 1) {
        if (Object.keys(userStore.userInfo).length === 0) {
            value = true;
        } else if (!userStore.userInfo.isCompanyAuth) {
            value = true;
        }
    }
    return value;
});

const currentCurrencyData = useCookie("currentCurrencyData");
const currency = computed(() => {
    const baseCurrency = commonStore.dollarSign || "￥";
    return isOverseas() ? currentCurrencyData.value?.symbol || baseCurrency : baseCurrency;
});
const price = computed(() => {
    let num = typeof props.modelValue === "number" ? String(props.modelValue) : props.modelValue;
    if (isOverseas() && currentCurrencyData.value?.rate) {
        num = moneyFormat((Number(num) * Number(currentCurrencyData.value.rate)).toString());
    }
    return splitPriceParts(num, props.completeDecimal);
});
</script>
<style lang="less" scoped>
.price {
    display: inline-flex;
    align-items: stretch;

    /* 子元素高度拉伸以匹配最高的子元素 */
    .text {
        font-size: 12px;
    }

    .util {
        margin-right: 2px;
    }

    .align-top {
        display: flex;
        /* 使用 Flexbox 布局 */
        align-items: flex-start;
        /* 使所有子元素沿底部对齐 */
    }

    .align-bottom {
        display: flex;
        /* 使用 Flexbox 布局 */
        align-items: flex-end;
        /* 使所有子元素沿底部对齐 */
    }
    .decimal{
        //font-size: 10px;
    }
}
</style>
