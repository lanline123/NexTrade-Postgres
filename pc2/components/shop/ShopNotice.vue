<template>
    <TotalContainer :module="module.frame">
        <div
            class="base-class"
            :style="{
                height: module.frame.itemHeight + 'px',
                borderRadius: module.frame.itemRadius + 'px',
                backgroundColor: module.frame.itemBackgroundColor
            }"
        >
            <el-image class="image-size" :src="imageFormat(module.icoPic?.picUrl)"></el-image>
            <div :style="{ color: module.frame.textColor }">{{ module.text }}</div>
        </div>
    </TotalContainer>
</template>
<script setup lang="ts">
import TotalContainer from "~/components/shop/src/TotalContainer.vue";

const props = defineProps({
    module: { type: Object, default: {} }
});
</script>
<style scoped lang="less">
.base-class {
    display: flex;
    flex-direction: row;
    align-items: center;
    gap: 10px;

    .image-size {
        width: 30px;
        height: 30px;
        margin-left: 20px;
    }
}
</style>
