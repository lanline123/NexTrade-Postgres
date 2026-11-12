<template>
    <client-only>
        <NuxtLink to="/cart/" class="cart-float-entry" :class="variant">
            <div class="icon-wrap">
                <i class="iconfont-pc icon-shiwu-gouwuche"></i>
                <span v-if="count > 0" class="badge">{{ displayCount }}</span>
            </div>
            <span class="label">{{ $t("购物车") }}</span>
        </NuxtLink>
    </client-only>
</template>

<script lang="ts" setup>
import { computed, onMounted } from "vue";
import { useCartStore } from "~/store/cart";

defineProps({
    variant: {
        type: String,
        default: "card"
    }
});

const cartStore: any = useCartStore();
const count = computed(() => Number(cartStore.cartCount) || 0);
const displayCount = computed(() => (count.value > 99 ? "99+" : count.value));

onMounted(() => {
    // 多实例并发时由 store 去重，只发一次请求
    cartStore.fetchCartCount().catch(() => {});
});
</script>

<style lang="less" scoped>
.cart-float-entry {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    text-decoration: none;

    .icon-wrap {
        position: relative;
        display: flex;
        align-items: center;
        justify-content: center;

        .badge {
            position: absolute;
            top: -6px;
            right: -12px;
            min-width: 16px;
            height: 16px;
            padding: 0 4px;
            box-sizing: border-box;
            line-height: 16px;
            text-align: center;
            font-size: 11px;
            color: #fff;
            background-color: #ff4400;
            border-radius: 9px;
        }
    }
}

/* 首页：透明卡片 + 分割线（跟随 ElevatorList，白底由父级提供） */
.cart-float-entry.flat {
    position: relative;
    gap: 2px;
    padding: 10px 10px;
    text-align: center;
    line-height: 20px;
    font-size: 12px;
    color: #333;

    .icon-wrap i {
        font-size: 20px;
    }

    &::after {
        content: "";
        position: absolute;
        left: 0;
        right: 0;
        bottom: 0;
        height: 0.5px;
        background: linear-gradient(to right, transparent, #c4c4c4, transparent);
    }

    &:hover {
        background-color: var(--general);
        color: var(--main-text);
        transition: 0.3s;

        .icon-wrap i,
        .label {
            color: var(--main-text);
        }
    }
}

/* 商品页/兜底：68×68 白卡（跟随 ElevatorProduct） */
.cart-float-entry.card {
    gap: 4px;
    width: 68px;
    height: 68px;
    font-size: 12px;
    color: #666666;
    background-color: #fff;
    border-radius: 6px;
    box-shadow: 0 4px 10px 0 rgba(0, 0, 0, 0.08);

    .icon-wrap i {
        font-size: 24px;
    }

    &:hover {
        color: var(--general);
        transition: 0.3s;
    }
}
</style>
