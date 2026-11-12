<template>
    <TotalContainer :module="module.frame">
        <Title :module="module.title"></Title>
        <div class="image-list">
            <OneRowX :size="2" v-if="module.picType == 1" :module="module"></OneRowX>
            <OneRowX :size="3" v-if="module.picType == 2" :module="module"></OneRowX>
            <OneRowX :size="4" v-if="module.picType == 3" :module="module"></OneRowX>
            <TwoRow2 v-if="module.picType == 4" :module="module"></TwoRow2>
            <Left1Right2 v-if="module.picType == 5" :module="module"></Left1Right2>
            <Left1Right3 v-if="module.picType == 6" :module="module"></Left1Right3>
            <Top1Bottom2 v-if="module.picType == 7" :module="module"></Top1Bottom2>
        </div>
    </TotalContainer>
</template>
<script setup lang="ts">
import { urlFormat } from "~/utils/format";
import TotalContainer from "~/components/shop/src/TotalContainer.vue";
import Title from "~/components/shop/src/Title.vue";
import OneRowX from "~/components/shop/src/OneRowX.vue";
import TwoRow2 from "~/components/shop/src/TwoRow2.vue";
import Left1Right2 from "~/components/shop/src/Left1Right2.vue";
import Left1Right3 from "~/components/shop/src/Left1Right3.vue";
import Top1Bottom2 from "~/components/shop/src/Top1Bottom2.vue";

const props = defineProps({
    module: { type: Object, default: {} }
});
</script>
<style scoped lang="less">
.image-list {
    box-sizing: border-box;
}
</style>
