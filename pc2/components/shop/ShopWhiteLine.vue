<template>
    <div
        class="big-link"
        :style="{
            paddingTop: module.frame.boxPaddingTop + 'px',
            paddingBottom: module.frame.boxPaddingBottom + 'px',
            paddingLeft: module.frame.boxPadding + 'px',
            paddingRight: module.frame.boxPadding + 'px'
        }"
    >
        <div class="link" :style="{ backgroundColor: module.frame.backgroundColor }">
            <div v-if="module.lineType == 2" :class="'link-style-' + module.lineStyle">
                <div class="item-line"></div>
            </div>
        </div>
    </div>
</template>
<script setup lang="ts">
const props = defineProps({
    module: { type: Object, default: {} }
});
</script>
<style scoped lang="less">
.big-link {
    display: flex;
    .link {
        flex: 1;
        padding: 15px 0;
        .link-style-1 {
            border-bottom: 1px solid #e5e5e5;
            width: 100%;
        }
        .link-style-2 {
            border-bottom: 1px dashed #e5e5e5;
            width: 100%;
        }
        .link-style-3 {
            border-bottom: 1px dotted #e5e5e5;
            width: 100%;
        }
    }
}
</style>
