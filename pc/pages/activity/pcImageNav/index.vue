<template>
    <TotalContainer
        v-if="module.picList && module.picList.length > 0"
        :module="totalContainerModule"
        :class="
            'module-ad-con module-image_nav ad-nav_type__' +
            module.navType +
            ' ad-nav_style__' +
            module.navStyle +
            ' ad-row_num__' +
            module.rowNum +
            ' ad-col_num__' +
            module.colNum +
            ' ad-pic_page_type__' +
            module.picPageType +
            ' ad-radio_style__' +
            module.radioStyle
        "
    >
        <Title :module="module.title"></Title>
        <XRowX v-if="module.navType == 1" :module="module"></XRowX>
        <CarouselNav v-if="module.navType == 2" :module="module"></CarouselNav>
    </TotalContainer>
</template>

<script lang="ts" setup>
/**
 * PC端装修图文导航组件
 * - 支持多行多列布局（navType=1）或轮播导航（navType=2）
 * - 可配置行数、列数、图片分页类型、圆角样式等
 * - 支持自定义轮播分页器颜色
 */
import { computed } from "vue";
import TotalContainer from "~/components/shop/src/TotalContainer.vue";
import Title from "~/pages/activity/components/Title.vue";
import XRowX from "~/components/shop/src/XRowX.vue";
import CarouselNav from "~/components/shop/src/CarouselNav.vue";

const props = defineProps({
    module: {
        type: Object,
        default: () => ({})
    }
});

const totalContainerModule = computed(() => {
    return {
        ...props.module?.frame
    };
});

const swiperPageColor = computed(() => {
    return props.module.swiperPageColor || "#333";
});
</script>
<style lang="less" scoped>
:deep(.swiper-pagination-bullet-active) {
    background: v-bind("swiperPageColor") !important;
}
</style>
