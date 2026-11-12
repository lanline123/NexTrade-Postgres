<template>
    <div>
        <CommonTopBar v-if="!hideHeader"></CommonTopBar>
        <div class="cart-top flex justify-center">
            <div class="cart-cont flex" :class="hideHeader ? 'justify-center' : 'justify-between'">
                <div class="logo" v-if="!hideHeader">
                    <NuxtLink exact-active-class="''" to="/">
                        <img :src="imageFormat(commonStore.shopLogo)" width="200" height="auto" alt="" />
                    </NuxtLink>
                </div>
                <div class="steps">
                    <Steps :current="props.step - 1"></Steps>
                </div>
            </div>
        </div>
    </div>
</template>
<script lang="ts" setup>
import { useCommonStore } from "@/store/common";
const commonStore = useCommonStore();
const props = defineProps({
    step: { type: Number, default: 0 },
    // 页面已由 CommonPageHeader 提供头部时传 true，仅渲染步骤条，避免与页面头部重复
    hideHeader: { type: Boolean, default: false }
});
</script>
<style lang="less" scoped>
.logo {
    min-height: 100px;
    display: flex;
    align-items: center;
    img {
        width: 190px;
        padding: 10px 0;
    }
}
.cart-top {
    background-color: #ffffff;
    position: relative;
    z-index: 79;
    border-bottom: 1px solid #eee;
    margin-bottom: 20px;

    .cart-cont {
        width: 1190px;
        padding: 0 0 0 0;
    }
}
.steps {
    padding-top: 20px;
}
</style>
