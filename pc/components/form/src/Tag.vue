<template>
    <div :class="['tag-style', { 'transparent-style': transparent, 'non-transparent-style': !transparent }]" :style="tagStyle">
        {{ text }}
    </div>
</template>
<script lang="ts" setup>
import { computed } from "vue";

const props = defineProps({
    text: String,
    color: {
        type: String,
        default: "#31c19e"
    },
    transparent: {
        type: Boolean,
        default: true
    }
});

const tagStyle = computed(() => {
    if (props.transparent) {
        return {
            color: props.color,
            border: `1px solid ${props.color}`,
            backgroundColor: "white"
        };
    } else {
        return {
            color: "white",
            backgroundColor: props.color,
            border: `1px solid ${props.color}`
        };
    }
});
</script>
<style lang="less" scoped>
.tag-style {
    display: inline-block; /* 使其按内容大小调整宽度 */
    padding: 0 3px; /* 设置内边距，增加文字与边框之间的距离 */
    border-radius: 2px; /* 圆角 */
    font-size: 12px;
    line-height: 14px;
    margin: 6px 3px 6px 6px;
    vertical-align: middle;
}
</style>
