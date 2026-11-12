<template>
    <TotalContainer :module="module.frame" :class="'module-ad-con module-white_blank'">
        <div class="white-blank-warp">
            <div class="white-blank-con" :style="styleFormat.backgroundColor + styleFormat.height" />
        </div>
    </TotalContainer>
</template>

<script lang="ts" setup>
/**
 * PC端装修空白辅助组件
 * - 用于在模块之间添加空白间距
 * - 支持自定义背景色和高度
 * - 默认高度为 30px
 */
import { computed } from "vue";
import TotalContainer from "~/components/shop/src/TotalContainer.vue";

const props = defineProps({
    module: {
        type: Object,
        default: () => ({})
    }
});

const styleFormat = computed(() => {
    return {
        backgroundColor: props.module.backgroundColor ? `background-color: ${props.module.backgroundColor};` : "",
        height: props.module.blankHeight ? `height: ${props.module.blankHeight}px;` : "height: 30px;"
    };
});
</script>
<style lang="less" scoped>
/*空白*/
.white-blank-warp {
    width: 100%;
    box-sizing: border-box;

    .white-blank-con {
        width: 100%;
        height: 30px;
        min-height: 10px;
    }
}
</style>
