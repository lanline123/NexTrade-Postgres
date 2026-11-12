<template>
    <CommonPageHeader />
    <search pageType="/list" />
    <CommonPageFooter />
</template>
<script lang="ts" setup>
/**
 * 商品列表页入口
 * - 复用搜索组件展示商品列表
 * - 通过 pageType="/list" 保证筛选/排序变更后停留在列表页路由
 */
import search from "~/pages/search/components/search.vue";
</script>
