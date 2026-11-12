<template>
    <div class="pc-header-runtime header-shell-MjI4MnNlbm8">
        <PcHeaderTopBarRuntime :style-type="1" :is-white="resolvedIsWhite" :config="config" />

        <template v-if="isCartPage">
            <PcHeaderCartContent :is-white="resolvedIsWhite" :page="page" :shop-search="shopSearch" :config="config" />
        </template>
        <div v-else class="pc-header-simple">
            <div class="pc-header-simple__header" :class="[{ 'pc-header-simple__header--white': resolvedIsWhite }, dividerClass]">
                <div class="pc-container">
                    <div class="pc-header" :style="headerVars">
                        <div ref="headerWrapperRef" class="pc-header__wrapper">
                            <div class="pc-header-home__left">
                                <div class="pc-header__logo-slot" :class="{ 'pc-header__logo-slot--wide': !showCategory }">
                                    <div class="pc-header__logo" :class="logoAlignClass">
                                        <div class="pc-header__logo-area">
                                            <NuxtLink exact-active-class="''" to="/" class="alv1 pc-header__logo-link">
                                                <img :src="logoSrc" :style="logoImgStyle" alt="" />
                                            </NuxtLink>
                                        </div>
                                    </div>
                                </div>
                                <div v-if="showCategory" class="category-nav" :style="categoryNavStyle">
                                    <AllCategoryNew :is-white="resolvedIsWhite"></AllCategoryNew>
                                </div>
                            </div>

                            <div class="pc-header-home__center">
                                <div class="pc-header__search-nav">
                                    <div ref="searchWrapRef" class="pc-header__search" :class="{ 'pc-header__search--accent': !!searchAccentColor }" :style="searchAccentStyle">
                                        <ShopSearchInput v-if="shopSearch" :page="page"></ShopSearchInput>
                                        <DefSearchNew v-if="!shopSearch" :is-white="resolvedIsWhite" :accent-color="searchAccentColor"></DefSearchNew>
                                    </div>
                                    <ul v-if="showMainNav" class="sub-nav sub-nav--stacked" :class="{ 'sub-nav--home': isHome }" :style="mainNavSpacingVars">
                                        <li v-for="(item, index) in commonStore.mainNav" :key="index">
                                            <CommonLink class="alv1" :item="item">{{ item.title ?? "" }}</CommonLink>
                                        </li>
                                    </ul>
                                </div>
                            </div>

                            <div class="pc-header-home__right" :class="{ 'pc-header-home__right--empty': !showRightCart }">
                                <div
                                    v-if="showRightCart"
                                    class="pc-header__cart"
                                    :class="{ 'pc-header__cart--pic': cartStyle === 2 }"
                                    role="button"
                                    tabindex="0"
                                    @click="goCart"
                                    @keydown.enter.prevent="goCart"
                                >
                                    <div class="pc-cart" :class="{ 'pc-cart--pic': cartStyle === 2, 'pc-cart--has-pic': cartStyle === 2 && !!cartPicThumb }">
                                        <template v-if="cartStyle === 2">
                                            <img v-if="cartPicThumb" class="pc-cart__pic" :src="cartPicThumb" alt="" />
                                            <div v-else class="pc-cart__pic-empty">{{ $t("购物车") }}</div>
                                        </template>
                                        <template v-else>
                                            <span class="pc-icon pc-icon--cart pc-cart__icon"></span>
                                            <span v-if="cartStore.cartCount > 0" class="pc-cart__count">{{ cartStore.cartCount }}</span>
                                            <span class="pc-cart__text">{{ $t("购物车") }}</span>
                                        </template>
                                    </div>
                                </div>
                            </div>
                        </div>
                    </div>
                </div>
            </div>
        </div>
    </div>
</template>

<script lang="ts" setup>
/**
 * PC 头部·简约风(styleType=1)主体（运行时）
 * 照搬 vortmall PcHeaderSimpleRuntime：分类用 AllCategoryNew、搜索用 DefSearchNew、热词 + logo 与搜索区高度同步(ResizeObserver)。
 * vort 专有依赖已替换为 NexTrade 等价：svg?component -> 内联图标组件、commonApi -> NexTrade api、热词无接口时不渲染。
 */
// ===================== Vue 核心 =====================
import { computed, ref } from "vue";

// ===================== 组件导入 =====================
import PcHeaderCartContent from "./PcHeaderCartContent.vue";
import AllCategoryNew from "~/components/common/components/AllCategoryNew.vue";
import DefSearchNew from "~/components/common/components/DefSearchNew.vue";
import ShopSearchInput from "~/pages/shop/src/ShopSearchInput.vue";
import PcHeaderTopBarRuntime from "./PcHeaderTopBarRuntime.vue";
import CommonLink from "~/components/common/Link.vue";

// ===================== 类型导入 =====================
import type { PcHeaderConfig } from "~/types/decorate/pcHeader";

// ===================== 工具函数 =====================
import { imageFormat, productImage, urlFormat } from "~/utils/format";

// ===================== Store =====================
import { useCommonStore } from "~/store/common";
import { useCartStore } from "~/store/cart";

// ===================== Props & Emits =====================
defineOptions({ name: "CommonPcHeaderSimpleRuntime" });

const props = defineProps<{
    isWhite: boolean;
    page: string;
    shopSearch: boolean;
    config: PcHeaderConfig;
}>();

// ===================== Store 实例 =====================
const commonStore = useCommonStore();
const cartStore = useCartStore();
const route = useRoute();

// ===================== 计算属性 =====================
const resolvedIsWhite = computed(() => Boolean(props.isWhite));
const isHome = computed(() => route.path === "/");
const isCartPage = computed(() => route.path === "/cart" || route.path.startsWith("/cart/"));

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

const showMainNav = computed(() => Number(props.config?.showMainNav ?? 1) !== 0);
const showCategory = computed(() => {
    if ((props.config?.showCategory ?? 1) === 0) return false;
    if (!isHome.value) return true;
    return (props.config?.showHomeCategory ?? 1) !== 0;
});
const categoryNavStyle = computed(() => {
    const mt = resolveFiniteNonNegativePx((props.config as unknown as Record<string, unknown>)?.categoryMarginTop) ?? 0;
    return {
        "--pc-header-category-mt": `${mt}px`
    } as Record<string, string>;
});
const showRightCart = computed(() => (props.config?.rightCartShow ?? 1) !== 0);
const cartStyle = computed(() => (props.config?.cartStyle ?? 1) as 1 | 2);

const cartPicThumb = computed(() => productImage(props.config?.cartPic));

// ===================== Logo 区域高度与搜索区同步 =====================
// 说明：以 DOM 可视矩形计算搜索区高度，并通过 CSS 变量只作用到 logo 区域，避免影响分类/右侧等布局。
const searchWrapRef = ref<HTMLElement | null>(null);
const headerWrapperRef = ref<HTMLElement | null>(null);
const logoAreaHeightPx = ref<number>(60);
const logoAreaShiftYPx = ref<number>(0);
const searchExtraBottomPx = ref<number>(0);
const mainNavOffsetLeftPx = ref<number | null>(null);
const mainNavOffsetRightPx = ref<number | null>(null);
let resizeObserver: ResizeObserver | null = null;
let rafId = 0;

function measureSearchBlockHeight(): void {
    if (!import.meta.client) return;
    const root = searchWrapRef.value;
    if (!root) return;

    const inputEl =
        root.querySelector<HTMLElement>(".top-search") ||
        root.querySelector<HTMLElement>(".search-bar") ||
        root.querySelector<HTMLElement>("input")?.closest<HTMLElement>(".top-search") ||
        null;
    const searchRect = root.getBoundingClientRect();
    const wrapperEl = headerWrapperRef.value;
    const wrapperRect = wrapperEl?.getBoundingClientRect();

    if (wrapperEl && wrapperRect && Number.isFinite(searchRect.left) && Number.isFinite(searchRect.right) && searchRect.right > searchRect.left) {
        const wrapperStyle = window.getComputedStyle(wrapperEl);
        const wrapperContentLeft = wrapperRect.left + (Number.parseFloat(wrapperStyle.paddingLeft) || 0);
        const wrapperContentRight = wrapperRect.right - (Number.parseFloat(wrapperStyle.paddingRight) || 0);
        mainNavOffsetLeftPx.value = Math.max(0, Math.round(searchRect.left - wrapperContentLeft));
        mainNavOffsetRightPx.value = Math.max(0, Math.round(wrapperContentRight - searchRect.right));
    } else {
        mainNavOffsetLeftPx.value = null;
        mainNavOffsetRightPx.value = null;
    }

    let top = Number.POSITIVE_INFINITY;
    let bottom = Number.NEGATIVE_INFINITY;

    const updateByRect = (el: HTMLElement | null) => {
        if (!el) return;
        const r = el.getBoundingClientRect();
        if (!Number.isFinite(r.top) || !Number.isFinite(r.bottom)) return;
        top = Math.min(top, r.top);
        bottom = Math.max(bottom, r.bottom);
    };

    updateByRect(inputEl || root);

    if (!Number.isFinite(top) || !Number.isFinite(bottom) || bottom <= top) {
        top = searchRect.top;
        bottom = searchRect.bottom;
    }
    const h = Math.ceil(bottom - top);
    logoAreaHeightPx.value = Math.max(40, h);
    searchExtraBottomPx.value = 0;

    if (wrapperRect && Number.isFinite(wrapperRect.top) && Number.isFinite(wrapperRect.height) && wrapperRect.height > 0) {
        const searchCenterY = (top + bottom) / 2;
        // 搜索+导航上下排布时，logo 仍按首行(60px)视觉中心对齐，避免被下方导航拉偏
        const firstRowHeight = Math.min(wrapperRect.height, 60);
        const wrapperCenterY = wrapperRect.top + firstRowHeight / 2;
        logoAreaShiftYPx.value = Math.round(searchCenterY - wrapperCenterY);
    } else {
        logoAreaShiftYPx.value = 0;
    }
}

function scheduleMeasure(): void {
    if (!import.meta.client) return;
    if (rafId) cancelAnimationFrame(rafId);
    rafId = requestAnimationFrame(() => {
        rafId = 0;
        nextTick(() => measureSearchBlockHeight());
    });
}

const headerVars = computed(() => {
    return {
        "--pc-header-logo-area-h": `${logoAreaHeightPx.value}px`,
        "--pc-header-logo-area-shift-y": `${logoAreaShiftYPx.value}px`,
        "--pc-header-search-extra-bottom": `${searchExtraBottomPx.value}px`,
        "--pc-header-padding-bottom": "0px"
    } as Record<string, string>;
});

onMounted(() => {
    scheduleMeasure();

    if (import.meta.client && typeof ResizeObserver !== "undefined") {
        resizeObserver = new ResizeObserver(() => scheduleMeasure());
        if (searchWrapRef.value) resizeObserver.observe(searchWrapRef.value);
        if (headerWrapperRef.value) resizeObserver.observe(headerWrapperRef.value);
    } else if (import.meta.client) {
        window.addEventListener("resize", scheduleMeasure, { passive: true });
    }
});

onBeforeUnmount(() => {
    if (rafId) cancelAnimationFrame(rafId);
    rafId = 0;
    if (resizeObserver && searchWrapRef.value) {
        resizeObserver.unobserve(searchWrapRef.value);
    }
    if (resizeObserver && headerWrapperRef.value) {
        resizeObserver.unobserve(headerWrapperRef.value);
    }
    resizeObserver = null;
    if (import.meta.client) {
        window.removeEventListener("resize", scheduleMeasure as EventListener);
    }
});

watch(
    () => [props.shopSearch, route.fullPath, isHome.value, showCategory.value, showRightCart.value],
    () => {
        scheduleMeasure();
    }
);

type BottomDividerStyle = "shadow" | "border" | "none";

function normalizeBottomDividerStyle(value: unknown, fallback: BottomDividerStyle): BottomDividerStyle {
    if (value == null) return fallback;
    if (typeof value === "number") {
        if (value === 1) return "shadow";
        if (value === 2) return "border";
        if (value === 0) return "none";
        return fallback;
    }

    const raw = String(value).trim().toLowerCase();
    if (["1", "shadow", "阴影"].includes(raw)) return "shadow";
    if (["2", "border", "line", "边框"].includes(raw)) return "border";
    if (["3", "none", "hidden", "no", "不显示", "无"].includes(raw)) return "none";
    return fallback;
}

type HeaderConfigRecord = Record<string, unknown>;

const dividerClass = computed(() => {
    const cfg = props.config as unknown as HeaderConfigRecord;
    const keys = [
        "bottomDividerType",
        "bottomDividerStyle",
        "headerBottomDividerStyle",
        "headerDividerStyle",
        "bottomLineStyle",
        "headerBottomLineStyle",
        "splitLineStyle"
    ];
    const key = keys.find((k) => cfg?.[k] != null);
    const style = normalizeBottomDividerStyle(key ? cfg[key] : undefined, "border");
    return `pc-header-divider--${style}`;
});

function resolveFiniteNonNegativePx(value: unknown): number | null {
    if (value == null) return null;
    const n = typeof value === "string" ? Number(value) : (value as number);
    if (!Number.isFinite(n)) return null;
    return Math.max(0, n);
}

function resolveMainNavSpacingPx(config: HeaderConfigRecord, dir: "top" | "bottom"): number | null {
    const keys =
        dir === "top"
            ? ["mainNavMarginTop", "mainNavTopMargin", "mainNavTopSpace", "mainNavSpaceTop", "mainNavPaddingTop", "mainNavTopPadding", "subNavMarginTop", "subNavTopMargin", "navMarginTop", "navTopMargin"]
            : ["mainNavMarginBottom", "mainNavBottomMargin", "mainNavBottomSpace", "mainNavSpaceBottom", "mainNavPaddingBottom", "mainNavBottomPadding", "subNavMarginBottom", "subNavBottomMargin", "navMarginBottom", "navBottomMargin", "componentMarginBottom"];

    for (const k of keys) {
        const v = resolveFiniteNonNegativePx(config?.[k]);
        if (v != null) return v;
    }
    return null;
}

const mainNavSpacingVars = computed(() => {
    const cfg = props.config as unknown as HeaderConfigRecord;
    const mt = resolveMainNavSpacingPx(cfg, "top");
    const mb = resolveMainNavSpacingPx(cfg, "bottom");

    const vars: Record<string, string> = {};
    vars["--pc-header-main-nav-mt"] = `${mt ?? 0}px`;
    if (mb != null) {
        vars["--pc-header-main-nav-mb"] = `${mb}px`;
    }
    if (mainNavOffsetLeftPx.value != null) {
        vars["--pc-header-nav-offset-left"] = `${mainNavOffsetLeftPx.value}px`;
    }
    if (mainNavOffsetRightPx.value != null) {
        vars["--pc-header-nav-offset-right"] = `${mainNavOffsetRightPx.value}px`;
    }
    return vars;
});

const searchAccentColor = computed(() => {
    if (Number(props.config?.searchColorType ?? 1) !== 2) return "";
    return String(props.config?.searchColor || "#1a1a1a");
});

const searchAccentStyle = computed(() => {
    if (!searchAccentColor.value) return {};
    return { "--pc-header-search-accent": searchAccentColor.value } as Record<string, string>;
});

// ===================== 事件处理方法 =====================
const goCart = (): void => {
    const url = urlFormat("/cart");
    if (!url) return;
    navigateTo(url);
};
</script>

<style lang="less" scoped>
@import "../_pcHeaderPreviewIcons.less";

.pc-header-runtime {
    width: 100%;
    /* 抬高整块头部，避免底部分割阴影被下方白底主内容盖住 */
    position: relative;
    z-index: 30;
}

/* 容器对齐新版首页主体 home-new-warp(max1600/min1264/padding38)，保证头部与下方装修模块左右对齐 */
.pc-container {
    margin: 0 auto;
    width: 100%;
    min-width: var(--pc-home-min-width, 1264px);
    max-width: var(--pc-home-max-width, 1600px);
    padding: 0 var(--pc-home-padding-x, 38px);
    box-sizing: border-box;
}

.pc-header-simple {
    background: #ffffff;
    min-width: var(--pc-home-min-width, 1264px);
}

.pc-header-simple__header {
    background: transparent;
    border-bottom: 0;
    box-shadow: none;
}

.pc-header-simple__header--white {
    background: #ffffff;
}

.pc-header-divider--shadow {
    position: relative;
    z-index: 1;
    box-shadow:
        0 2px 12px 0 #0000000a,
        0 4px 6px 0 #0000000f;
}

.pc-header-divider--border {
    border-bottom: 1px solid #ededed;
}

.pc-header-divider--none {
    border-bottom: 0;
    box-shadow: none;
}

.pc-header {
    padding: 16px 0 var(--pc-header-padding-bottom, 0px);
}

.pc-header__wrapper {
    display: grid;
    align-items: start;
    grid-template-columns: max-content minmax(0, 1fr) max-content;
    column-gap: 16px;
    height: auto;
    min-height: 60px;

    .pc-header-home__left {
        display: flex;
        align-items: center;
        justify-content: flex-start;
        gap: 16px;
        height: 60px;
        min-width: 0;
        overflow: visible;
    }

    .pc-header-home__center,
    .pc-header-home__right,
    .pc-header__logo,
    .pc-header__search,
    .pc-header__right {
        min-width: 0;
    }

    .pc-header-home__right {
        display: flex;
        align-items: center;
        justify-content: center;
        height: 60px;
        width: 252px;
        min-width: 252px;
        max-width: 252px;
        overflow: visible;
    }

    .pc-header-home__right--empty {
        pointer-events: none;
    }

    .pc-header__logo {
        width: auto;
    }

    .pc-header__search {
        width: 100%;
    }

    .pc-header-home__center {
        display: flex;
        align-items: flex-start;
    }

    .pc-header__search-nav {
        width: 100%;
        min-width: 0;
        display: flex;
        flex-direction: column;
    }

    .pc-header__cart {
        justify-content: center;
        min-width: 0;
    }
}

.pc-header__logo {
    flex-shrink: 0;
    width: 100%;
    display: flex;
    align-items: center;
    gap: 16px;
    overflow: visible;

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

.pc-header__logo-slot {
    width: 160px;
    min-width: 160px;
    max-width: 160px;
    flex-shrink: 0;
}

.pc-header__logo-slot--wide {
    width: 250px;
    min-width: 250px;
    max-width: 250px;
}

.pc-header__logo-area {
    height: 60px;
    display: flex;
    align-items: center;
    flex-shrink: 0;
    overflow: hidden;
    transform: none;
}

.pc-header__logo-link {
    max-width: 100%;
    height: 100%;
    display: inline-flex;
    align-items: center;
}

.pc-header__logo-link img {
    max-width: 100%;
    max-height: 100%;
    display: block;
}

.category-nav {
    min-width: 74px;
    height: 100%;
    margin-top: var(--pc-header-category-mt, 0px);
    white-space: nowrap;
    flex-shrink: 0;
}

.pc-header__search {
    flex: none;
    min-height: 60px;
    height: 60px;
    min-width: 0;
    display: flex;
    align-items: center;
    position: relative;
    overflow: visible;

    &--accent {
        :deep(.top-search),
        :deep(.top-search:hover) {
            border-color: var(--pc-header-search-accent, #c1c1c1) !important;
        }

        :deep(.search-form .search-submit) {
            background-color: var(--pc-header-search-accent, #1a1a1a) !important;
        }
    }
}

.pc-header__cart {
    flex-shrink: 0;
    min-width: clamp(96px, 14vw, 120px);
    display: flex;
    justify-content: flex-end;
}

.pc-header__cart--pic {
    min-width: clamp(140px, 24vw, 180px);
}

.pc-cart {
    position: relative;
    display: inline-flex;
    align-items: center;
    gap: 8px;
    cursor: pointer;
    user-select: none;
    color: #1a1a1a;

    &--pic {
        width: 180px;
        height: 60px;
        border-radius: 6px;
        overflow: hidden;
        background: #f3f4f6;
        border: 1px solid #ededed;
        display: flex;
        align-items: center;
        justify-content: center;
    }

    &--has-pic {
        background: transparent;
        border: 0;
    }

    &:hover {
        color: var(--primary-dark-2);
    }

    .pc-cart__icon {
        width: 20px;
        height: 20px;
    }

    &__pic {
        width: 100%;
        height: 100%;
        object-fit: cover;
        display: block;
    }

    &__pic-empty {
        font-size: 12px;
        color: #9ca3af;
    }

    &__count {
        position: absolute;
        top: -6px;
        left: 10px;
        min-width: 16px;
        height: 16px;
        line-height: 16px;
        padding: 0 3px;
        border-radius: 100px;
        background: #cc4b11;
        border: 1px solid #fff;
        color: #fff;
        font-size: 10px;
        display: inline-flex;
        align-items: center;
        justify-content: center;
        box-sizing: border-box;
    }

    &__text {
        font-size: 14px;
        font-weight: 500;
    }
}

.sub-nav {
    display: flex;
    align-items: center;
    justify-content: flex-start;
    min-height: 42px;
    height: auto;
    gap: 45px;
    row-gap: 8px;
    flex-wrap: wrap;
    margin-left: 0;
    margin-right: 0;
    margin-top: var(--pc-header-main-nav-mt, 0px);
    margin-bottom: var(--pc-header-main-nav-mb, 0px);
    width: 100%;
    max-width: none;
    padding-top: 0;
    padding-bottom: 0;
    padding-left: var(--pc-header-nav-offset-left, 0px);
    padding-right: var(--pc-header-nav-offset-right, 0px);
    box-sizing: border-box;

    li {
        text-align: center;
        font-size: 14px;
        color: #1a1a1a;
        font-family: "Trebuchet MS", "Lucida Sans Unicode", "Lucida Grande", "Lucida Sans", Arial, sans-serif;

        :deep(a) {
            color: #1a1a1a !important;
            text-decoration: none;
        }

        :deep(a:hover) {
            color: #1a1a1a !important;
            text-decoration: underline;
        }
    }
}

.sub-nav--stacked {
    min-height: 42px;
    gap: 28px;
    padding-left: 0;
    padding-right: 0;
}
</style>
