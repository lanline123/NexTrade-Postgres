<template>
    <div
        :style="{
            paddingTop: `${module.boxPaddingTop ?? module.innerPadding ?? 0}px`,
            paddingBottom: `${module.boxPaddingBottom ?? module.innerPadding ?? 0}px`,
            paddingLeft: `${module.boxPadding ?? module.innerPadding ?? 0}px`,
            paddingRight: `${module.boxPadding ?? module.innerPadding ?? 0}px`
        }"
    >
        <div class="base-class" :style="{ backgroundColor: module.backgroundColor, borderRadius: module.boxRadius + 'px', overflow: module.boxRadius ? 'hidden' : undefined }">
            <slot></slot>
        </div>
    </div>
</template>
<script setup lang="ts">
defineProps({
    module: { type: Object, default: () => ({}) }
});
</script>
<style scoped lang="less"></style>
