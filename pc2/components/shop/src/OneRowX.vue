<template>
    <div class="one-row-style">
        <template v-for="(item, index) in module.picList" :key="index">
            <NuxtLink
                :style="{ padding: module.imgPadding + 'px', width: 100 / size + '%' }"
                target="_blank"
                :to="urlFormat(item.picLink)"
                class="image-container"
            >
                <el-image
                    :class="[Number(module.picRadioStyle) === 2 ? 'image-box-radius' : '']"
                    fit="fill"
                    class="image-box"
                    :src="imageFormat(item.picUrl)"
                ></el-image>
            </NuxtLink>
        </template>
    </div>
</template>

<script setup lang="ts">
const props = defineProps({
    module: { type: Object, default: {} },
    size: { type: Number, default: 1 }
});
import { urlFormat } from "~/utils/format";
</script>

<style scoped lang="less">
.one-row-style {
    display: flex;

    flex-wrap: wrap;

    .image-container {
        padding: 0;
        text-align: center; /* 图片居中显示 */
        box-sizing: border-box;
        display: flex;
    }
}

.image-box {
    flex: 1;
}

.image-box-radius {
    border-radius: 8px;
    overflow: hidden;

    :deep(.el-image__inner) {
        border-radius: 8px;
    }
}
</style>
