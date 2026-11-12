/**
 * PC 装修模块注册表（前台 · 新套，标准版）
 * type 使用 kebab-case，与 Admin 对齐。
 * banner2 / group-banner / category-nav3 为跨境外贸风首页件，随 overseas.pcStyle 整文件裁剪。
 */

import type { Component } from "vue";
import PcBanner2 from "~/decorate/components/pc/PcBanner2.vue";
import PcGroupBanner from "~/decorate/components/pc/PcGroupBanner.vue";
import PcCategoryNav3 from "~/decorate/components/pc/PcCategoryNav3.vue";
import PcCategoryNavWrapper from "~/decorate/components/pc/PcCategoryNavWrapper.vue";
import PcImageAd from "~/decorate/components/pc/PcImageAd.vue";
import PcImageNav from "~/decorate/components/pc/PcImageNav.vue";
import PcImageSquareAd from "~/decorate/components/pc/PcImageSquareAd.vue";
import PcImageHotarea from "~/decorate/components/pc/PcImageHotarea.vue";
import PcNotice from "~/decorate/components/pc/PcNotice.vue";
import PcTitleAd from "~/decorate/components/pc/PcTitleAd.vue";
import PcQuickNav from "~/decorate/components/pc/PcQuickNav.vue";
import PcHomeHero from "~/decorate/components/pc/PcHomeHero.vue";

export type PcDecorateModuleType =
    | "pc-image-ad"
    | "pc-image-nav"
    | "pc-image-square-ad"
    | "pc-image-hotarea"
    | "pc-product"
    | "pc-divider"
    | "pc-spacer"
    | "pc-notice"
    | "pc-title-ad"
    | "pc-home-quick-nav"
    | "pc-home-coupon"
    | "pc-home-groupon"
    | "pc-home-seckill"
    | "pc-product-group"
    | "pc-home-hero"
    | "pc-category-nav"
    | "pc-recommend-product"
    | "pc-banner2"
    | "pc-group-banner"
    | "pc-category-nav3"
    ;

export interface PcModuleDefinition {
    type: PcDecorateModuleType;
    getComponent: () => Promise<{ default: Component }>;
}

const PC_MODULE_REGISTRY: Record<PcDecorateModuleType, PcModuleDefinition> = {
    "pc-image-ad": {
        type: "pc-image-ad",
        getComponent: () => import("~/decorate/components/pc/PcImageAd.vue")
    },
    "pc-image-nav": {
        type: "pc-image-nav",
        getComponent: () => import("~/decorate/components/pc/PcImageNav.vue")
    },
    "pc-image-square-ad": {
        type: "pc-image-square-ad",
        getComponent: () => import("~/decorate/components/pc/PcImageSquareAd.vue")
    },
    "pc-image-hotarea": {
        type: "pc-image-hotarea",
        getComponent: () => import("~/decorate/components/pc/PcImageHotarea.vue")
    },
    "pc-product": {
        type: "pc-product",
        getComponent: () => import("~/decorate/components/pc/PcProduct.vue")
    },
    "pc-divider": {
        type: "pc-divider",
        getComponent: () => import("~/decorate/components/pc/PcDivider.vue")
    },
    "pc-spacer": {
        type: "pc-spacer",
        getComponent: () => import("~/decorate/components/pc/PcSpacer.vue")
    },
    "pc-notice": {
        type: "pc-notice",
        getComponent: () => import("~/decorate/components/pc/PcNotice.vue")
    },
    "pc-title-ad": {
        type: "pc-title-ad",
        getComponent: () => import("~/decorate/components/pc/PcTitleAd.vue")
    },
    "pc-home-quick-nav": {
        type: "pc-home-quick-nav",
        getComponent: () => import("~/decorate/components/pc/PcQuickNav.vue")
    },
    "pc-home-coupon": {
        type: "pc-home-coupon",
        getComponent: () => import("~/decorate/components/pc/PcCoupon.vue")
    },
    "pc-home-groupon": {
        type: "pc-home-groupon",
        getComponent: () => import("~/decorate/components/pc/PcGroupon.vue")
    },
    "pc-home-seckill": {
        type: "pc-home-seckill",
        getComponent: () => import("~/decorate/components/pc/PcSeckill.vue")
    },
    "pc-product-group": {
        type: "pc-product-group",
        getComponent: () => import("~/decorate/components/pc/PcProductGroup.vue")
    },
    "pc-home-hero": {
        type: "pc-home-hero",
        getComponent: () => import("~/decorate/components/pc/PcHomeHero.vue")
    },
    "pc-category-nav": {
        type: "pc-category-nav",
        // CategoryNew 使用 modelValue，wrapper 适配 renderer 的 module 入参
        getComponent: () => import("~/decorate/components/pc/PcCategoryNavWrapper.vue")
    },
    "pc-recommend-product": {
        type: "pc-recommend-product",
        getComponent: () => import("~/decorate/components/pc/PcRecommendProduct.vue")
    },
    "pc-banner2": {
        type: "pc-banner2",
        getComponent: () => import("~/decorate/components/pc/PcBanner2.vue")
    },
    "pc-group-banner": {
        type: "pc-group-banner",
        getComponent: () => import("~/decorate/components/pc/PcGroupBanner.vue")
    },
    "pc-category-nav3": {
        type: "pc-category-nav3",
        getComponent: () => import("~/decorate/components/pc/PcCategoryNav3.vue")
    }
};

// 首屏同步组件（避免异步 chunk 往返）；取数件（product/coupon/groupon/seckill）与 divider/spacer 走懒加载
const PC_CRITICAL_COMPONENT_REGISTRY: Partial<Record<PcDecorateModuleType, Component>> = {
    "pc-image-ad": PcImageAd,
    "pc-image-nav": PcImageNav,
    "pc-image-square-ad": PcImageSquareAd,
    "pc-image-hotarea": PcImageHotarea,
    "pc-notice": PcNotice,
    "pc-title-ad": PcTitleAd,
    "pc-home-quick-nav": PcQuickNav,
    "pc-home-hero": PcHomeHero,
    "pc-category-nav": PcCategoryNavWrapper,
    "pc-banner2": PcBanner2,
    "pc-group-banner": PcGroupBanner,
    "pc-category-nav3": PcCategoryNav3
};

export function getPcModuleDefinition(type: string): PcModuleDefinition | null {
    return PC_MODULE_REGISTRY[type as PcDecorateModuleType] ?? null;
}

export function getPcCriticalComponent(type: string): Component | null {
    return PC_CRITICAL_COMPONENT_REGISTRY[type as PcDecorateModuleType] ?? null;
}
