<template>
    <div class="pc-header-cart-content" :class="dividerClass">
        <div class="container">
            <div class="pc-header-cart__wrapper">
                <div class="pc-header-cart__left">
                    <div class="pc-header__logo" :class="logoAlignClass">
                        <NuxtLink exact-active-class="''" to="/" class="alv1 pc-header__logo-link">
                            <img :src="logoSrc" :style="logoImgStyle" alt="" />
                        </NuxtLink>
                    </div>
                    <div class="cart-title">
                        <span class="text">{{ $t("购物车") }}</span>
                        <span v-if="cartStore.cartCount > 0" class="count">({{ cartStore.cartCount }})</span>
                    </div>
                </div>

                <div class="pc-header-cart__right">
                    <div class="pc-header__search" :style="{ '--search-btn-text': `'${$t('搜索')}'` }">
                        <ShopSearchInput v-if="shopSearch" :page="page"></ShopSearchInput>
                        <DefSearchNew v-if="!shopSearch" :is-white="isWhite" :enable-guess="false"></DefSearchNew>
                    </div>
                </div>
            </div>
        </div>
    </div>
</template>

<script lang="ts" setup>
/**
 * 购物车页头部主体（运行时）
 * 仅 logo + “购物车”标题 + 搜索框，无分类/导航；逻辑口径参考 vortmall PcHeaderCartContent，
 * 相比 vortmall 省略了头部“全选/删除/移入收藏/清空”等购物车操作与 o2o 配送地区（保持精简）。
 */
import { computed } from "vue";
import type { PcHeaderConfig } from "~/types/decorate/pcHeader";
import { imageFormat, productImage } from "~/utils/format";
import { useCommonStore } from "~/store/common";
import { useCartStore } from "~/store/cart";
import DefSearchNew from "~/components/common/components/DefSearchNew.vue";
import ShopSearchInput from "~/pages/shop/src/ShopSearchInput.vue";

defineOptions({ name: "PcHeaderCartContent" });

const props = defineProps<{
    isWhite: boolean;
    page: string;
    shopSearch: boolean;
    config: PcHeaderConfig;
}>();

const commonStore = useCommonStore();
const cartStore = useCartStore();

const logoSrc = computed(() => productImage(props.config?.logoPic) || imageFormat(commonStore.shopLogo));

const logoImgStyle = computed(() => {
    const w = props.config?.logoWidth ?? 110;
    const h = props.config?.logoHeight ?? 40;
    return {
        width: `${w}px`,
        height: `${h}px`,
        objectFit: "contain"
    } as const;
});

const logoAlignClass = computed(() => {
    const align = Number(props.config?.logoAlign ?? 1);
    if (align === 2) return "logo--center";
    if (align === 3) return "logo--right";
    return "logo--left";
});

// Admin 口径：bottomDividerType 0=不展示 / 1=阴影 / 2=边框；购物车页默认边框
const dividerClass = computed(() => {
    const raw = props.config?.bottomDividerType;
    if (raw === 0) return "pc-header-divider--none";
    if (raw === 1) return "pc-header-divider--shadow";
    return "pc-header-divider--border";
});
</script>

<style lang="less" scoped>
.pc-header-cart-content {
    background: #ffffff;
    min-width: var(--pc-home-min-width, 1264px);
    width: 100%;
    position: relative;
    z-index: 1;
}

.pc-header-divider--shadow {
    box-shadow:
        0 2px 12px 0 rgba(0, 0, 0, 0.0392),
        0 4px 6px 0 rgba(0, 0, 0, 0.0588);
}

.pc-header-divider--border {
    border-bottom: 1px solid #ededed;
}

.pc-header-divider--none {
    border-bottom: 0;
    box-shadow: none;
}

/* 容器对齐新版首页主体 home-new-warp(max1600/min1264/padding38) */
.container {
    margin: 0 auto;
    width: 100%;
    min-width: var(--pc-home-min-width, 1264px);
    max-width: var(--pc-home-max-width, 1600px);
    padding: 0 var(--pc-home-padding-x, 38px);
    box-sizing: border-box;
}

.pc-header-cart__wrapper {
    display: flex;
    align-items: center;
    justify-content: space-between;
    height: 80px;
}

.pc-header-cart__left {
    display: flex;
    align-items: center;
    gap: 18px;
}

.pc-header__logo {
    flex-shrink: 0;
    display: flex;
    align-items: center;

    &.logo--left {
        justify-content: flex-start;
    }
    &.logo--center {
        justify-content: center;
    }
    &.logo--right {
        justify-content: flex-end;
    }
}

.pc-header__logo-link {
    display: inline-flex;
    align-items: center;

    img {
        max-height: 100%;
        display: block;
    }
}

.cart-title {
    display: flex;
    align-items: center;
    gap: 6px;

    .text {
        font-size: 22px;
        font-weight: 500;
        color: #1a1a1a;
    }

    .count {
        font-size: 22px;
        font-weight: 500;
        line-height: 1;
        color: #999;
    }
}

.pc-header-cart__right {
    display: flex;
    align-items: center;
    width: 400px;
}

.pc-header__search {
    width: 100%;

    /* DefSearchNew 自带白底圆角搜索框；购物车页覆盖为：主题色“搜索”文字按钮 + 左侧放大镜（照搬 vortmall 购物车页） */
    :deep(.top-search) {
        border-width: 1px;
        border-color: var(--general);
    }

    :deep(.search-submit) {
        display: flex;
        align-items: center;
        justify-content: center;
        width: 68px;
        background: var(--general) !important;
        background-color: var(--general) !important;
        color: var(--main-text);
        font-size: 0;

        svg {
            display: none;
        }

        &::after {
            content: var(--search-btn-text, "搜索");
            font-size: 14px;
            font-weight: 500;
            color: #fff;
            font-family: inherit;
        }
    }

    :deep(.search-form) {
        position: relative;
        border-width: 1px;
        padding-left: 28px;
        box-sizing: border-box;

        &::before {
            content: "";
            position: absolute;
            left: 12px;
            top: 50%;
            width: 12px;
            height: 12px;
            border: 1.5px solid #666;
            border-radius: 50%;
            box-sizing: border-box;
            transform: translateY(-60%);
            z-index: 1;
            pointer-events: none;
        }

        &::after {
            content: "";
            position: absolute;
            left: 22px;
            top: 22px;
            width: 6px;
            height: 1.5px;
            background: #666;
            transform: rotate(45deg);
            transform-origin: left center;
            z-index: 1;
            pointer-events: none;
        }
    }

    :deep(.row-middle) {
        width: 100%;
        min-width: 0;
    }

    :deep(.search-input) {
        padding-left: 0;
    }
}
</style>
