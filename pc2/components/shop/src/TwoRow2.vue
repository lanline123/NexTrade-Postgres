<template>
    <div class="image-list">
        <div class="image-list-min">
            <NuxtLink target="_blank" :to="urlFormat(module.picList[0].picLink)" :style="{ padding: module.imgPadding + 'px' }" class="image">
                <el-image fit="fill" class="image-box" :src="imageFormat(module.picList[0].picUrl)"></el-image>
            </NuxtLink>
            <NuxtLink target="_blank" :to="urlFormat(module.picList[1].picLink)" :style="{ padding: module.imgPadding + 'px' }" class="image">
                <el-image fit="fill" class="image-box" :src="imageFormat(module.picList[1].picUrl)"></el-image>
            </NuxtLink>
        </div>
        <div class="image-list-min">
            <NuxtLink target="_blank" :to="urlFormat(module.picList[2].picLink)" :style="{ padding: module.imgPadding + 'px' }" class="image">
                <el-image fit="fill" class="image-box" :src="imageFormat(module.picList[2].picUrl)"></el-image>
            </NuxtLink>
            <NuxtLink target="_blank" :to="urlFormat(module.picList[3].picLink)" :style="{ padding: module.imgPadding + 'px' }" class="image">
                <el-image fit="fill" class="image-box" :src="imageFormat(module.picList[3].picUrl)"></el-image>
            </NuxtLink>
        </div>
    </div>
</template>
<script setup lang="ts">
const props = defineProps({
    module: { type: Object, default: {} }
});
import { urlFormat } from "~/utils/format";
</script>
<style scoped lang="less">
.image-list {
    display: flex;

    .image-list-min {
        display: flex;
        flex: 1;
        justify-content: space-between;
        flex-direction: column;
        .image {
            flex: 1;
            box-sizing: border-box;
            display: flex;
        }
    }
}
.image-box {
    flex: 1;
}
</style>
