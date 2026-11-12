<template>
    <div class="one-row-style">
        <div class="image-container" v-for="(item, index) in module.picList" :key="index">
            <el-image fit="fill" class="image-box" :src="imageFormat(item.picUrl)"></el-image>
            <NuxtLink
                target="_blank"
                :to="urlFormat(hot.link)"
                v-for="hot in item.hotarea"
                class="hot-btn"
                :style="{ height: hot.height * 2.4 + 'px', width: hot.width * 2.4 + 'px', left: hot.left * 2.4 + 'px', top: hot.top * 2.4 + 'px' }"
            >
            </NuxtLink>
        </div>
    </div>
</template>

<script setup lang="ts">
import { urlFormat } from "~/utils/format";

const props = defineProps({
    module: { type: Object, default: {} }
});
</script>

<style scoped lang="less">
.one-row-style {
    display: flex;

    flex-wrap: wrap;

    .image-container {
        width: 100%;
        position: relative;
        text-align: center; /* 图片居中显示 */
        box-sizing: border-box;
        display: flex;
        flex-direction: column;
        gap: 6px;

        .hot-btn {
            position: absolute;
            cursor: pointer;
        }
    }
}

.image-box {
    display: flex;
    flex: 1;
}
</style>
