<template>
    <TotalContainer v-if="module.picList && module.picList.length > 0" :module="module.frame">
        <Title :module="module.title"></Title>
        <OneRowX :size="1" v-if="module.picType == 1" :module="module"></OneRowX>
        <Carousel v-if="module.picType == 2" :module="module"></Carousel>
        <LateralSlip size="big" v-if="module.picType == 3" :module="module"></LateralSlip>
        <LateralSlip size="small" v-if="module.picType == 4" :module="module"></LateralSlip>
        <OneRowX :size="2" v-if="module.picType == 5" :module="module"></OneRowX>
        <OneRowX :size="3" v-if="module.picType == 6" :module="module"></OneRowX>
        <OneRowX :size="4" v-if="module.picType == 7" :module="module"></OneRowX>
    </TotalContainer>
</template>
<script setup lang="ts">
/**
 * PC端装修图片广告组件
 * - 支持多种图片展示类型（picType 1-7）
 * - picType 1: 单图；2: 轮播；3: 大图横向滑动；4: 小图横向滑动
 * - picType 5/6/7: 分别为一行2/3/4张图片
 */
import TotalContainer from "~/components/shop/src/TotalContainer.vue";
import Title from "~/components/shop/src/Title.vue";
import OneRowX from "~/components/shop/src/OneRowX.vue";
import LateralSlip from "~/components/shop/src/LateralSlip.vue";
import Carousel from "~/components/shop/src/Carousel.vue";

const props = defineProps({
    module: { type: Object, default: {} }
});
</script>
<style scoped lang="less"></style>
