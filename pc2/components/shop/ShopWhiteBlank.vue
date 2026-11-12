<template>
    <div :style="{ height: module.blankHeight + 'px', backgroundColor: module.backgroundColor }"></div>
</template>
<script setup lang="ts">
const props = defineProps({
    module: { type: Object, default: {} }
});
</script>
<style scoped lang="less"></style>
