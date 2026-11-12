<template>
    <NuxtLink :target="item.isBlank === 1 ? '_blank' : '_self'" :to="item.link ? urlFormat(item.link) : ''">
        <slot></slot>
    </NuxtLink>
</template>

<script setup lang="ts">
const props = defineProps({
    item: { type: Object, default: () => ({}) }
});
</script>

<style lang="less" scoped></style>
