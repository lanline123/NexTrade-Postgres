<template>
    <div v-if="isOverseas()" class="pc-header-runtime header-shell-MjI4MnNlbm8">
        <PcHeaderTopBarRuntime :style-type="2" :is-white="resolvedIsWhite" :config="config" />

        <template v-if="isCartPage">
            <PcHeaderCartContent :is-white="resolvedIsWhite" :page="page" :shop-search="shopSearch" :config="config" />
        </template>
        <div v-else class="page-container" :class="[dividerClass]">
            <div class="page-header-inner" :style="headerVars">
                <div ref="headerWrapperRef" class="page-c-warap">
                    <div class="page-header-home__left">
                        <div class="logo-slot" :class="{ 'logo-slot--wide': !showCategory }">
                            <div class="logo" :class="logoAlignClass">
                                <div class="logo-area">
                                    <NuxtLink exact-active-class="''" to="/" class="alv1 logo-link">
                                        <img :src="logoSrc" :style="logoImgStyle" alt="" />
                                    </NuxtLink>
                                </div>
                            </div>
                        </div>
                        <div v-if="showCategory" class="category-nav" :style="categoryNavStyle">
                            <AllCategoryNew :is-white="resolvedIsWhite"></AllCategoryNew>
                        </div>
                    </div>
                    <div class="page-header-home__center">
                        <div class="search-nav-box">
                            <div ref="searchWrapRef" class="search-bar" :class="{ 'search-bar--accent': !!searchAccentColor }" :style="searchAccentStyle">
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
                    <div class="page-header-home__right">
                        <div v-if="isOverseas()" class="language-bar">
                            <LanguageChangeNew class="item" :is-white="resolvedIsWhite" lang-label-mode="localeCode"></LanguageChangeNew>
                        </div>
                        <div class="mine-bar">
                            <Mine :is-white="resolvedIsWhite"></Mine>
                        </div>
                        <div v-if="showRightCart" class="car-bar">
                            <NuxtLink exact-active-class="''" to="/cart" @click="handleLink('/cart')">
                                <div class="content" :class="{ 'content--pic': cartStyle === 2 }">
                                    <template v-if="cartStyle === 2">
                                        <img v-if="cartPicThumb" class="cart-img" :src="cartPicThumb" alt="" />
                                        <div v-else class="cart-img-empty">{{ $t("购物车") }}</div>
                                    </template>
                                    <template v-else>
                                        <div class="icon">
                                            <CartIcon width="20" height="20"></CartIcon>
                                        </div>
                                        <div v-if="cartStore.cartCount > 0" class="count">{{ cartStore.cartCount }}</div>
                                        <div class="txt">{{ $t("购物车") }}</div>
                                    </template>
                                </div>
                            </NuxtLink>
                        </div>
                    </div>
                </div>
            </div>
        </div>
    </div>
</template>

<script lang="ts" setup>
/**
 * PC 头部·跨境风(styleType=2)主体（运行时）
 * 照搬 vortmall PcHeaderOverseasRuntime；vort 专有依赖替换为 NexTrade 等价：
 * svg?component -> 内联图标组件、Mine/LanguageChangeNew 用 el-popover/element 适配版。
 */
import { computed, ref } from "vue";

import DefSearchNew from "~/components/common/components/DefSearchNew.vue";
import CartIcon from "~/components/icons/CartIcon.vue";
import AllCategoryNew from "~/components/common/components/AllCategoryNew.vue";
import Mine from "~/components/common/components/Mine.vue";
import PcHeaderCartContent from "./PcHeaderCartContent.vue";
import LanguageChangeNew from "~/components/locate/LanguageChangeNew.vue";
import ShopSearchInput from "~/pages/shop/src/ShopSearchInput.vue";
import PcHeaderTopBarRuntime from "./PcHeaderTopBarRuntime.vue";
import CommonLink from "~/components/common/Link.vue";

import type { PcHeaderConfig } from "~/types/decorate/pcHeader";

import { imageFormat, productImage } from "~/utils/format";
import { isOverseas } from "@/utils/util";

import { useCommonStore } from "~/store/common";
import { useCartStore } from "~/store/cart";

defineOptions({ name: "CommonPcHeaderOverseasRuntime" });

const props = defineProps<{
    isWhite: boolean;
    page: string;
    shopSearch: boolean;
    config: PcHeaderConfig;
}>();

const commonStore = useCommonStore();
const cartStore = useCartStore();
const route = useRoute();
const router = useRouter();

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

const searchAccentColor = computed(() => {
    if (Number(props.config?.searchColorType ?? 1) !== 2) return "";
    return String(props.config?.searchColor || "#1a1a1a");
});

const searchAccentStyle = computed(() => {
    if (!searchAccentColor.value) return {};
    return { "--pc-header-search-accent": searchAccentColor.value } as Record<string, string>;
});

// ===================== Logo 区域高度与搜索区同步 =====================
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
    const style = normalizeBottomDividerStyle(key ? cfg[key] : undefined, "shadow");
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

const handleLink = async (url: string): Promise<void> => {
    if (url === router.currentRoute.value.path) {
        location.reload();
    }
};
</script>

<style lang="less" scoped>
@import "../_pcHeaderPreviewIcons.less";

.pc-header-runtime {
    width: 100%;
    position: relative;
    z-index: 30;
}

.page-container {
    box-sizing: border-box;
    width: 100%;
    min-width: var(--pc-home-min-width, 1264px);
    height: auto;
    background: #fff;
    box-shadow: none;
    border-bottom: 0;
    position: relative;
    /* 低于 benefit，避免底部分割阴影的 blur 顶穿到顶栏；整块抬层交给 .pc-header-runtime */
    z-index: 1;

    .page-header-inner {
        padding: 16px 0 var(--pc-header-padding-bottom, 0px);
    }

    .page-c-warap {
        flex: 1;
        display: grid;
        align-items: start;
        margin: 0 auto;
        height: auto;
        min-height: 60px;
        min-width: 1008px;
        max-width: var(--pc-home-max-width, 1866px);
        width: 100%;
        padding: 0 var(--pc-home-padding-x, 40px);
        box-sizing: border-box;
        grid-template-columns: max-content minmax(0, 1fr) max-content;
        column-gap: var(--pc-home-gap, 16px);

        .page-header-home__left {
            display: flex;
            align-items: center;
            justify-content: flex-start;
            gap: 16px;
            height: 60px;
            min-width: 0;
            overflow: visible;
        }

        .page-header-home__center {
            min-width: 0;
            display: flex;
            align-items: flex-start;
        }

        .search-nav-box {
            width: 100%;
            min-width: 0;
            display: flex;
            flex-direction: column;
        }

        .page-header-home__right {
            display: flex;
            align-items: center;
            justify-content: flex-end;
            gap: 14px;
            height: 60px;
            min-width: 0;
            overflow: visible;
        }

        .logo {
            flex-shrink: 0;
            width: 100%;
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

        .logo-slot {
            width: 160px;
            min-width: 160px;
            max-width: 160px;
            flex-shrink: 0;
        }

        .logo-slot--wide {
            width: 250px;
            min-width: 250px;
            max-width: 250px;
        }

        .logo-area {
            height: 60px;
            display: flex;
            align-items: center;
            flex-shrink: 0;
            overflow: hidden;
            transform: none;
        }

        .logo-link {
            max-width: 100%;
            height: 100%;
            display: inline-flex;
            align-items: center;
        }

        .logo-link img {
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

        .search-bar {
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

        .language-bar {
            display: flex;
            align-items: center;
            min-width: 0;
            height: 100%;

            :deep(.lang-box) {
                height: 20px;
                line-height: 20px;

                .lang-text {
                    line-height: 20px;
                }

                .arrow {
                    align-self: center;
                    margin-bottom: 0;
                }
            }
        }

        .mine-bar {
            display: flex;
            align-items: center;
            min-width: 0;
            height: 100%;
            white-space: nowrap;
            position: relative;
            z-index: 1;

            :deep(.popover-mine) {
                z-index: 9999;
            }

            :deep(.category-wrap .bar-wrap .content) {
                height: 20px;
                align-items: center;

                .txt {
                    line-height: 20px;
                }

                .icon.ml {
                    align-self: center;
                    margin-bottom: 0;
                }
            }
        }

        .car-bar {
            display: flex;
            align-items: center;
            min-width: 0;
            height: 100%;
            white-space: nowrap;

            .content {
                position: relative;
                cursor: pointer;
                display: flex;
                height: 40px;
                width: auto;

                .cart-img {
                    align-self: center;
                    width: 40px;
                    height: 40px;
                    object-fit: contain;
                    border-radius: 4px;
                }

                .cart-img-empty {
                    align-self: center;
                    width: 100%;
                    height: 40px;
                    border-radius: 4px;
                    display: flex;
                    align-items: center;
                    justify-content: center;
                    font-size: 12px;
                    color: #9ca3af;
                    background: #f3f4f6;
                    border: 1px solid #ededed;
                    box-sizing: border-box;
                    user-select: none;
                }

                .icon {
                    align-self: center;
                    width: 20px;
                    height: 20px;
                    border-radius: 4px;
                    background: transparent;
                    color: #1a1a1a;
                }

                .txt {
                    align-self: center;
                    margin-left: 6px;
                    font-size: 14px;
                    font-weight: 500;
                    color: #1a1a1a;
                }

                .count {
                    box-sizing: border-box;
                    align-items: center;
                    background: #cc4b11;
                    border-radius: 100px;
                    border: 1px solid #fff;
                    color: #fff;
                    font-size: 10px;
                    justify-content: center;
                    top: 3px;
                    left: 8px;
                    min-width: 16px;
                    padding: 0 3px;
                    position: absolute;
                    text-align: center;
                    display: flex;
                    height: 16px;
                    line-height: 16px;
                }
            }
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
        min-width: 0;
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
                position: relative;
                &::after {
                    content: "";
                    display: block;
                    height: 1.2px;
                    width: 100%;
                    position: absolute;
                    left: 0;
                    bottom: 0px;
                }
            }

            :deep(a:hover) {
                &::after {
                    background: #1a1a1a;
                }
            }
        }
    }

    .sub-nav--stacked {
        min-height: 42px;
        gap: 28px;
        padding-left: 0;
        padding-right: 0;
    }
}

.page-container.pc-header-divider--shadow {
    box-shadow:
        0 2px 12px 0 #0000000a,
        0 4px 6px 0 #0000000f;
}

.page-container.pc-header-divider--border {
    border-bottom: 1px solid #ededed;
    box-shadow: none;
}

.page-container.pc-header-divider--none {
    border-bottom: 0;
    box-shadow: none;
}
</style>
