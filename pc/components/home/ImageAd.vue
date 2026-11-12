<template>
    <div style="width: 100%">
        <div class="module-pic-list" :style="boxstyle">
            <div class="module-pic-list-item" v-for="(item, index) in modelValue.picList" :key="index">
                <NuxtLink :to="urlFormat(item.picLink)" target="_blank">
                    <img class="module-img" :alt="item?.picTitle" loading="lazy" :title="item?.picTitle" :src="imageFormat(item.picUrl)" />
                </NuxtLink>
            </div>
        </div>
    </div>
</template>
<script lang="ts" setup>
import { urlFormat, imageFormat } from "@/utils/format";
const props = defineProps({
    modelValue: { type: Object }
});

const boxstyle = computed(() => {
    return `padding-top: ${props.modelValue?.boxPaddingTop}px; padding-bottom: ${props.modelValue?.boxPaddingBottom}px;`;
});
</script>
<style lang="less" scoped>
.module-img {
    width: 100%;
    height: auto;
}
.module-img :deep(img) {
    width: 100%;
    height: auto;
}
</style>
