<!--
  积分兑换商品详情页入口
  - 动态路由页面，根据商品ID展示积分兑换商品详情
  - 复用商品详情组件，通过 productType="exchange" 切换为兑换模式
-->
<template>
    <div>
        <Product productType="exchange" :ui-version="uiVersion" />
    </div>
</template>
<script lang="ts" setup>
/**
 * 积分兑换商品详情页入口
 * - 动态路由页面，根据商品ID展示积分兑换商品详情
 * - 复用商品详情组件，通过 productType="exchange" 切换为兑换模式
 */
import { computed } from "vue";
import Product from "@/pages/item/src/Product.vue";
import { getProductDetailStyle } from "~/api/common";

// 用 useAsyncData 复用 SSR 结果并缓存，避免每次进商详重复请求及 SSR/CSR 水合不一致
const { data: styleType } = await useAsyncData("productDetailStyle", () => getProductDetailStyle());

const uiVersion = computed(() => {
    const style = Number(styleType.value);
    if (style === 3) return "overseas";
    if (style === 2) return "new";
    return "old";
});
</script>
