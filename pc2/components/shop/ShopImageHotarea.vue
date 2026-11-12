<template>
    <TotalContainer :module="module.frame">
        <Title :module="module.title"></Title>
        <HotImage :module="module"></HotImage>
    </TotalContainer>
</template>
<script setup lang="ts">
import TotalContainer from "~/components/shop/src/TotalContainer.vue";
import Title from "~/components/shop/src/Title.vue";
import XRowX from "~/components/shop/src/XRowX.vue";
import HotImage from "~/components/shop/src/HotImage.vue";
const props = defineProps({
    module: { type: Object, default: {} }
});
</script>
<style scoped lang="less"></style>
