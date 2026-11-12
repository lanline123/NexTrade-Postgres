<template>
    <div class="item-img-a">
        <div class="item-img-bg" :style="`background-image: url(${imageFormat(src ?? '')});`" />
        <el-image class="item-img" fit="fill" :src="imageFormat(src ?? '')" alt="" />
    </div>
</template>

<script setup lang="ts">
import { imageFormat } from "~/utils/format";

defineProps({
    src: String
});
</script>

<style lang="less" scoped>
.item-img-a {
    height: 100%;
    width: 100%;
    position: relative;

    .item-img-bg {
        position: absolute;
        top: 0;
        left: 0;
        background-position: center;
        background-size: cover;
        height: 100%;
        width: 100%;
        overflow: hidden;
    }
}

.item-img {
    width: 100%;
    opacity: 0;
}
</style>
