<template>
    <div>
        <Product :ui-version="uiVersion" />
    </div>
</template>
<script lang="ts" setup>
/**
 * 商品详情页入口
 * - 动态路由页面，根据商品ID展示商品详情
 * - 实际内容由 Product 子组件渲染
 * - 通过后台「商品页风格」配置切换 UI：
 *     样式一(1)=经典版(old)
 *     样式二(2)=新版(new)
 *     样式三(3)=跨境版(overseas)  —— 预留，等后台「商品页风格」下发 3 即生效
 * - 本地开发可通过 ?uiStyle=overseas|new|old 预览对应样式
 */
import { computed } from "vue";
import Product from "@/pages/item/src/Product.vue";
import { getProductDetailStyle } from "~/api/common";

// 用 useAsyncData 复用 SSR 结果并缓存，避免每次进商详重复请求及 SSR/CSR 水合不一致
const { data: styleType } = await useAsyncData("productDetailStyle", () => getProductDetailStyle());
const route = useRoute();

const uiVersion = computed(() => {
    if (import.meta.dev) {
        const previewStyle = route.query.uiStyle;
        if (previewStyle === "old" || previewStyle === "new" || previewStyle === "overseas") return previewStyle;
    }
    const style = Number(styleType.value);
    if (style === 3) return "overseas";
    if (style === 2) return "new";
    return "old";
});
</script>
