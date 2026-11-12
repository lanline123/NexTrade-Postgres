<template>
    <div class="tag" :class="{ active: active }">
        {{ $t(txt) }}
    </div>
</template>

<script lang="ts" setup>
// ===================== Props & Emits =====================
const props = defineProps({
    txt: {
        type: String,
        default: ""
    },
    active: {
        type: Boolean,
        default: false
    }
});
</script>

<style lang="less" scoped>
.tag {
    width: fit-content;
    max-width: 214px;
    overflow: hidden;
    text-overflow: ellipsis;
    background: #f7f8fc;
    padding: 8px 12px;
    font-size: 14px;
    border-radius: 4px;
    color: #505259;
    margin-right: 8px;
    margin-bottom: 8px;
    cursor: pointer;
    border: 1px solid #f7f8fc;
    &.active {
        border: 0.5px solid var(--main-bg);
        background-color: var(--primary-light-9);
        color: var(--main-bg);
    }
    &:hover {
        color: var(--main-bg);
        border: 1px solid #f7f8fc;
    }
}
</style>
