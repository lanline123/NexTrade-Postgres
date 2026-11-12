<template>
    <div class="one-row-style">
        <template v-for="(item, index) in displayList" :key="index">
            <NuxtLink
                :style="{ padding: module.imgPadding + 'px', width: 100 / module.rowNum + '%' }"
                target="_blank"
                :to="urlFormat(item.picLink)"
                class="image-container"
            >
                <el-image
                    v-if="module.navStyle != 3"
                    :class="[Number(module.radioStyle) === 2 ? 'image-box-radius' : '']"
                    fit="fill"
                    class="image-box"
                    :src="imageFormat(item.picUrl)"
                ></el-image>
                <div v-if="module.navStyle != 2" class="pic-title">{{ item.picTitle }}</div>
            </NuxtLink>
        </template>
    </div>
</template>

<script setup lang="ts">
import { computed } from "vue";
import { urlFormat } from "~/utils/format";

const props = defineProps({
    module: { type: Object, default: () => ({}) }
});

const displayList = computed(() => {
    const rowNum = Number(props.module.rowNum) || 4;
    const colNum = Number(props.module.colNum) || 1;
    const maxItems = rowNum * colNum;
    return (props.module.picList || []).slice(0, maxItems);
});
</script>

<style scoped lang="less">
.one-row-style {
    display: flex;

    flex-wrap: wrap;

    .image-container {
        text-align: center; /* 图片居中显示 */
        box-sizing: border-box;
        display: flex;
        flex-direction: column;
        gap: 6px;
        .pic-title {
            height: 20px;
            font-size: 12px;
            line-height: 1.5;
            white-space: nowrap; /* 不换行 */
            overflow: hidden; /* 隐藏超出容器的内容 */
            text-overflow: ellipsis; /* 超出部分用省略号表示 */
        }
    }
}

.image-box {
    display: flex;
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
