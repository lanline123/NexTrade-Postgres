<template>
    <div class="header" :class="{ 'is-classic': classic }" :style="headerStyle">
        <div class="wrapper login-header-Z2NkdTIyODI">
            <div class="logo">
                <NuxtLink to="/" class="alv1">
                    <img :src="imageFormat(commonStore?.shopLogo ?? '')" :style="logoImgStyle" alt="" />
                </NuxtLink>
            </div>
            <div v-if="showLogin" class="info">
                {{ $t("您好，欢迎光临{0}！", [commonStore.shopName]) }}
                <template v-if="type == 'register'">
                    <NuxtLink class="font-color" to="/member/login">{{ $t("已有账号？去登录！") }}</NuxtLink>
                </template>
                <template v-else-if="commonStore.shopRegClosed === 0">
                    <NuxtLink class="font-color" to="/member/register/">{{ $t("没有账号？注册新用户！") }}</NuxtLink>
                </template>
            </div>
        </div>
    </div>
</template>
<script setup lang="ts">
import { useCommonStore } from "~/store/common";
import { imageFormat } from "@/utils/format";
const props = defineProps({
    showLogin: {
        type: Boolean,
        default: true
    },
    type: {
        type: String,
        default: "login"
    },
    /** 头部高度，单位 px（来源：PC 登录页装修 headerHeight） */
    height: {
        type: Number,
        default: 70
    },
    /** 是否显示头部阴影（来源：PC 登录页装修 headerShadow） */
    shadow: {
        type: Boolean,
        default: true
    },
    /** Logo 宽度，单位 px（来源：PC 登录页装修 logoWidth） */
    logoWidth: {
        type: Number,
        default: 140
    },
    /** Logo 高度，单位 px（来源：PC 登录页装修 logoHeight） */
    logoHeight: {
        type: Number,
        default: 40
    },
    /** 经典版头部：内容限宽 1190px 居中（对齐 BaseLogin 布局），否则整宽 */
    classic: {
        type: Boolean,
        default: false
    }
});
const commonStore: any = useCommonStore();
commonStore.loadNav();

// 头部阴影，对齐样式内默认阴影
const HEADER_SHADOW = "0 2px 12px 0 #0000000a, 0 4px 6px 0 #0000000f";

// 头部高度/阴影按装修配置动态绑定
const headerStyle = computed(() => ({
    height: `${props.height}px`,
    boxShadow: props.shadow ? HEADER_SHADOW : "none",
    // 经典版(O2O)用 1px 底边框还原老头部；现代版(跨境)不加，仍用阴影
    borderBottom: props.classic ? "1px solid #ddd" : "none"
}));

// Logo 尺寸：固定宽 + object-fit: contain 防变形；logoHeight 为 0/未设时高度自适应（还原老版仅固定宽度）
const logoImgStyle = computed(() => ({
    maxWidth: `${props.logoWidth}px`,
    maxHeight: `${props.logoHeight}px`,
    objectFit: "contain" as const
}));
</script>
<style scoped lang="less">
.header {
    background: #fff;
    display: flex;
    justify-content: center;
    position: relative;
    z-index: 10;
    min-width: var(--pc-home-min-width, 1264px);
    &:after,
    :before {
        box-sizing: border-box;
        border: 0 solid #e5e7eb;
    }

    .wrapper {
        width: 100%;
        display: flex;
        justify-content: space-between;
        box-sizing: border-box;
        padding: 15px 40px;

        .logo {
            display: flex;
            align-items: center;

            img {
                // 宽高由内联 style 控制（装修 logoWidth/logoHeight），此处仅保留防变形
                object-fit: contain;
            }
        }

        .info {
            min-width: 300px;
            color: #666666;
            gap: 10px;
            display: flex;

            .help {
                text-decoration: underline;
                color: #666666;
            }
        }
    }

    // 经典版：头部内容限宽 1190px 居中（对齐 BaseLogin 头部布局）
    &.is-classic .wrapper {
        width: 1190px;
        padding: 15px 10px;
        // logo 宽高统一走装修 logoWidth/logoHeight，不再单独覆盖
    }
}
</style>
