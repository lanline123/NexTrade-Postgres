<!--
  PC端嵌入H5移动端页面
  - 在PC端通过iframe嵌入H5移动端页面
  - 自动检测设备类型，移动端设备自动跳转到H5页面
  - 支持通过URL参数传递路径和查询参数
  - 监听窗口大小变化和设备方向变化
-->
<template>
    <iframe v-if="!isMobile && isClient" :src="h5Url" class="mobile-iframe" frameborder="0" allowfullscreen></iframe>
</template>

<script setup lang="ts">
import { useCommonStore } from "@/store/common";

definePageMeta({
    layout: false
});

const commonStore = useCommonStore();

const isClient = import.meta.client;

const h5Url = computed(() => {
    if (!isClient) return "";

    let path = "";

    let query = "";

    let src = "";

    if (window.location.search) {
        const searchParams = new URLSearchParams(window.location.search);

        // 显式来源：卖家端等其他 H5 跳转时带上自身完整地址，避免被写死成买家端 /mobile
        if (searchParams.has("src")) {
            src = searchParams.get("src") || "";

            searchParams.delete("src");
        }

        if (searchParams.has("path")) {
            path = searchParams.get("path") || path;

            searchParams.delete("path");
        }

        query = searchParams.toString();
    }

    // 仅放行同源或买家端 H5 域名，防止 iframe 嵌入任意外部地址
    if (src) {
        try {
            const allowOrigins = [window.location.origin];

            if (commonStore.h5Domain) allowOrigins.push(new URL(commonStore.h5Domain).origin);

            if (allowOrigins.includes(new URL(src).origin)) return src;
        } catch {
            // 非法地址则忽略，回落到默认买家端逻辑
        }
    }

    let url = commonStore.h5Domain ? `${commonStore.h5Domain}${path}?${query}` : `${window.location.origin}/mobile${path}?${query}`;

    return url;
});

const isMobile = ref(false);

const handlerToMobile = () => {
    if (!isClient) return;
    window.location.href = h5Url.value;
};

const checkMobile = () => {
    if (!isClient) return false;

    const userAgent = navigator.userAgent;
    const mobileKeywords = /Android|webOS|iPhone|iPad|iPod|BlackBerry|IEMobile|Opera Mini/i;
    const isMobileDevice = mobileKeywords.test(userAgent);

    return isMobileDevice;
};

const handleResize = () => {
    if (!isClient) return;

    const mobile = checkMobile();
    if (mobile && !isMobile.value) {
        handlerToMobile();
    }
    isMobile.value = mobile;
};

onMounted(() => {
    if (checkMobile()) {
        return handlerToMobile();
    }

    window.addEventListener("resize", handleResize);
    window.addEventListener("orientationchange", handleResize);
});

onUnmounted(() => {
    window.removeEventListener("resize", handleResize);
    window.removeEventListener("orientationchange", handleResize);
});
</script>

<style lang="less" scoped>
.mobile-iframe {
    display: block;
    margin: 0 auto;
    width: 100%;
    min-width: 375px;
    max-width: 458px;
    height: 100vh;
    box-sizing: border-box;
    overflow: visible;
}

// 移动端样式
@media (max-width: 768px) {
    .mobile-iframe {
        max-width: 100%;
        min-width: 100%;
    }
}
</style>
