<template>
    <div class="app-empty empty-shell-MjI4MnZzanU">
        <el-empty :description="description" />
    </div>
</template>

<script lang="ts" setup>
/**
 * 通用空状态组件：包裹 el-empty，统一空数据展示。
 */
defineProps({
    description: { type: String, default: "" }
});
</script>

<style lang="less" scoped>
.app-empty {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 100%;
    padding: 40px 0;
}
</style>
