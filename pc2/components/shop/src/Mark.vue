<template>
    <ShopQualification :shop-info="shopInfo" />
</template>
<script setup lang="ts">
/**
 * @description 装修店铺信息栏「查看资质」浮层
 * - 评分 + 企业资质，复用 ShopQualification
 */
import ShopQualification from "~/components/shop/src/ShopQualification.vue";

defineProps({
    shopInfo: { type: Object, default: {} }
});
</script>
