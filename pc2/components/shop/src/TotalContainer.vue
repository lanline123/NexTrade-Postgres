<template>
    <div
        class="home-warp"
        :style="{
            paddingTop: module.boxPaddingTop + 'px',
            paddingBottom: module.boxPaddingBottom + 'px',
            paddingLeft: module.boxPadding + 'px',
            paddingRight: module.boxPadding + 'px'
        }"
    >
        <div class="base-class" :style="{ backgroundColor: module.backgroundColor, borderRadius: module.boxRadius + 'px' }">
            <slot></slot>
        </div>
    </div>
</template>
<script setup lang="ts">
const props = defineProps({
    module: { type: Object, default: {} }
});
</script>
<style scoped lang="less">
.ds {
    background-color: rgba(129, 88, 73, 0.68);
}
</style>
