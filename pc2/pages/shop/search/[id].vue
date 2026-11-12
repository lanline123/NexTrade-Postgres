<template>
    <ShopSearch ui-version="new" />
</template>
<script lang="ts" setup>
/**
 * @description 店铺内搜索页路由入口
 * - 动态路由 [id].vue 用于捕获店铺ID
 * - ?ui=new 切换新版 UI，默认旧版
 */
import { computed } from "vue";
import ShopSearch from "~/pages/shop/search/src/ShopSearch.vue";
</script>
