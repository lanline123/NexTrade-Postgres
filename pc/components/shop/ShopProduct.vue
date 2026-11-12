<template>
    <TotalContainer :module="temp">
        <Title :module="module.title"></Title>
        <OneRowProductX :size="module.style" :productList="productList" :module="module" v-if="[1, 2, 3].includes(Number(module.style))"></OneRowProductX>
        <OneBig2Small :productList="productList" :module="module" v-if="module.style == 4"></OneBig2Small>
        <OneRowProductX direction="land" :productList="productList" :module="module" v-if="module.style == 5"></OneRowProductX>
        <OneRowProductX direction="slide" :productList="productList" :module="module" v-if="module.style == 6"></OneRowProductX>
        <OneRow3Carousel :productList="productList" :module="module" v-if="module.style == 7"></OneRow3Carousel>
    </TotalContainer>
</template>
<script setup lang="ts">
import Title from "~/components/shop/src/Title.vue";
import TotalContainer from "~/components/shop/src/TotalContainer.vue";
import OneRowProductX from "~/components/shop/product/OneRowProductX.vue";
import OneBig2Small from "~/components/shop/product/OneBig2Small.vue";
import OneRow3Carousel from "~/components/shop/product/OneRow3Carousel.vue";
import { useComProParams } from "~/hooks";

const props = defineProps({
    module: { type: Object, default: {} },
    decorateId: { type: Number, default: 0 },
    moduleIndex: { type: Number, default: 0 },
    shopId: { type: Number, default: -1 }
});

const { productList } = useComProParams(props.shopId > -1 ? { ...props.module.products, shopId: props.shopId } : props.module.products);

const temp = computed(() => {
    let temps: any = props.module;
    temps.frame.backgroundColor = props.module.backgroundColor;
    return temps;
});
</script>
<style scoped lang="less"></style>
