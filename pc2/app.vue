<template>
    <div class="app app-shell-dHRmdzIyODI" :class="{ 'not-zh': !isZh && isOverseas(), 'is-member-page': isMemberPage }" :style="[themeStore.themeColor, homeWidthVars]">
        <el-config-provider :locale="locale">
            <NuxtPage page-key="static" />
        </el-config-provider>
        <ChooseStyle v-if="isDemo() && !route.path.includes('install') && !route.path.includes('pcMobile') && !route.path.includes('chat') && !route.path.includes('/shop/license')"></ChooseStyle>
        <PrivacyConsent
            v-if="isOverseas() && isDemo() && !route.path.includes('install') && !route.path.includes('pcMobile') && !route.path.includes('chat')"
        />
        <GlobalSideTools v-if="showSideTools" />
    </div>
</template>
<script lang="ts" setup>
import { useThemeStore } from "@/store/theme";
import { useCommonStore } from "@/store/common";
import { useI18nStore } from "@/store/i18n";
import { useCurrencyStore } from "@/store/currency";
import { useDecorateRuntimeStore } from "@/store/decorateRuntime";
// @ts-ignore
import zhCn from "element-plus/dist/locale/zh-cn.min.mjs";
// @ts-ignore
import en from "element-plus/dist/locale/en.min.mjs";
import ChooseStyle from "@/components/common/ChooseStyle.vue";
import GlobalSideTools from "@/components/common/GlobalSideTools.vue";
import PrivacyConsent from "@/components/common/PrivacyConsent.vue";
import { isDemo, isMobileBrowser, isOverseas } from "@/utils/util";
import { useRoute } from "vue-router";

useHead({
    meta: [
        {
            name: "viewport",
            content: "width=device-width, initial-scale=1.0, maximum-scale=1.0, user-scalable=no"
        }
    ]
});

const themeStore = useThemeStore();
const commonStore = useCommonStore();
const i18nStore = useI18nStore();
const currencyStore = useCurrencyStore();

if (isOverseas()) {
    // SSR 必须先确定语言，后续基础配置、装修、导航及首页接口才能携带一致的语言码。
    await i18nStore.init();
    currencyStore.fetchCurrencyList();
}

const isZh = computed(() => i18nStore.XLocaleCode?.includes("zh"));

// 输出实际页面语言，供搜索引擎和读屏软件识别。
useHead({
    htmlAttrs: {
        lang: computed(() => String(i18nStore.XLocaleCode || "zh-CN"))
    }
});

const locale = computed(() => {
    if (isOverseas()) {
        return isZh.value ? zhCn : en;
    } else {
        return zhCn;
    }
});

const route = useRoute();

// 会员中心内容区宽度覆盖挂钩，样式见 assets/css/member/member-width.less
const isMemberPage = computed(() => route.path === "/member" || route.path.startsWith("/member/"));

/** 精确路由匹配：命中根路由本身或其子路由（避免 includes 宽泛误命中） */
const isUnderRoute = (path: string, prefix: string) => path === prefix || path.startsWith(`${prefix}/`);

/**
 * 兜底悬浮侧栏显隐
 * - 首页 ElevatorList、商品页 ElevatorProduct 已内置，不重复挂
 * - 跨境搜索页另挂 ElevatorList，排除 GST；/list 不排除，避免空窗
 */
const showSideTools = computed(() => {
    // 归一化路径，避免 // 导致首页判断失败、兜底购物车与电梯购物车重复
    const path = (route.path || "/").replace(/\/+/g, "/") || "/";
    if (["/install", "/pcMobile", "/chat", "/join", "/shop/license"].some((p) => isUnderRoute(path, p))) return false;
    if (path === "/") return false;
    if (["/search", "/list", "/category", "/item"].some((p) => isUnderRoute(path, p))) return false;
    if (isOverseas()) {
        if (["/product", "/cart"].some((p) => isUnderRoute(path, p))) return false;
    }
    return true;
});

const previewId = computed(() => {
    return route.query.previewId ? Number(route.query.previewId) : 0;
});

const handlerToMobile = () => {
    if (!isMobileBrowser() || commonStore.autoRedirect !== 1) {
        return;
    }

    const baseUrl = (commonStore.h5Domain || `${window.location.origin}/mobile`).trim();
    let mobileUrl: URL;
    try {
        mobileUrl = new URL(baseUrl, window.location.origin);
    } catch {
        mobileUrl = new URL(`${window.location.origin}/mobile`);
    }

    // 配置错误时不要把 PC 根页重定向回自身；PC 和 H5 共用域名时也避免
    // `/mobile` 被回落到 PC 壳后形成自重定向循环。
    const normalizePath = (path: string) => {
        const normalized = path.replace(/\/+$/, "");
        return normalized || "/";
    };
    const currentUrl = new URL(window.location.href);
    const currentPath = normalizePath(currentUrl.pathname);
    if (mobileUrl.origin === currentUrl.origin) {
        if (normalizePath(mobileUrl.pathname) === "/") {
            mobileUrl.pathname = "/mobile";
        }
        const mobilePath = normalizePath(mobileUrl.pathname);
        if (currentPath === mobilePath || currentPath.startsWith(`${mobilePath}/`)) {
            return;
        }
    }

    const pathname = location?.pathname || "";

    const itemMatch = pathname.match(/\/item\/(.+)$/);
    const productSn = itemMatch?.[1];

    if (productSn) {
        const mobileBasePath = mobileUrl.pathname.replace(/\/+$/, "");
        mobileUrl.pathname = `${mobileBasePath}/pages/product/index`;
        mobileUrl.search = `?sn=${encodeURIComponent(productSn)}`;
    }

    window.location.replace(mobileUrl.toString());
};

const decorateRuntimeStore = useDecorateRuntimeStore();

/**
 * 全站内容宽度分流：首页新版装修走 1264–1600 流式，旧版/未配置回退固定 1190。
 * 在根节点就近赋值 --pc-home-*，统一覆盖各处 fallback，使 header、.container 容器、footer
 * 全站随首页新旧版一致切换（信号来源见 decorateRuntime.isHomeNewStyle）。
 */
const homeWidthVars = computed<Record<string, string>>(() =>
    decorateRuntimeStore.isHomeNewStyle
        ? { "--pc-home-max-width": "1600px", "--pc-home-min-width": "1264px", "--pc-home-padding-x": "38px" }
        : { "--pc-home-max-width": "1190px", "--pc-home-min-width": "auto", "--pc-home-padding-x": "0px" }
);

/**
 * previewId 变化后重取头部装修：app.vue 的 setup 只在首屏跑一次，客户端切风格只改 query，
 * 不在这里重取的话宽度变量会残留上一套风格（如经典风沿用现代风的 1600）。
 */
watch(previewId, (pid) => {
    // 演示站切风格由 ChooseStyle 统一预取并与首页模块同帧提交，这里不插手
    if (decorateRuntimeStore.styleSwitching) return;
    decorateRuntimeStore.reloadGlobalPcHeaderConfig(pid > 0 ? pid : undefined);
});

try {
    await commonStore.loadBaseConfig({ previewId: previewId.value > 0 ? previewId.value : undefined });

    await commonStore.loadThemeSettings();
    // 全站加载 PC 头部装修配置：后台配了头部装修则全站启用新版头部，否则回退旧版；SSR 阶段完成避免首屏闪动
    await decorateRuntimeStore.ensureGlobalPcHeaderConfig(previewId.value > 0 ? previewId.value : undefined);
    // 如果是自定义页面不调整到首页，走自定义页面逻辑
    if (import.meta.client && route.path !== "/activity") {
        handlerToMobile();
    }

    useHead({
        title: () => commonStore.shopTitle + (commonStore.poweredByStatus != 1 ? "-powered by nextrade" : ""),
        meta: [
            { name: "description", content: () => commonStore.shopDesc },
            { name: "keywords", content: () => commonStore.shopKeywords }
        ],
        link: [
            {
                rel: "icon",
                type: "image/x-icon",
                href: imageFormat(commonStore.icoImg)
            }
        ]
    });
} catch (error) {
    console.log(error);
}

onMounted(() => {
    // 禁止双指缩放
    document.addEventListener(
        "touchstart",
        function (event) {
            if (event.touches.length > 1) {
                event.preventDefault();
            }
        },
        { passive: false }
    );

    // 禁止双击缩放
    let lastTouchEnd = 0;
    document.addEventListener(
        "touchend",
        function (event) {
            const now = new Date().getTime();
            if (now - lastTouchEnd <= 300) {
                event.preventDefault();
            }
            lastTouchEnd = now;
        },
        false
    );
    document.addEventListener("gesturestart", function (event) {
        event.preventDefault();
    });
});
</script>
