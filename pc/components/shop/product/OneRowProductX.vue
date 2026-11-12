<template>
    <div class="one-row-style" :class="[direction == 'slide' ? 'no' : 'can']">
        <template v-for="(item, index) in productList" :key="index">
            <ProductCardLand v-if="direction == 'land'" :module="module" :size="size" :product="item" :product-style="styleMap[item.productId]"></ProductCardLand>
            <ProductCardSlide v-else-if="direction == 'slide'" :module="module" :product="item" :product-style="styleMap[item.productId]"></ProductCardSlide>
            <ProductCard v-else :module="module" :size="size" :product="item" :product-style="styleMap[item.productId]"></ProductCard>
        </template>
    </div>
</template>

<script setup lang="ts">
import ProductCard from "~/components/shop/product/ProductCard.vue";
import ProductCardLand from "~/components/shop/product/ProductCardLand.vue";
import ProductCardSlide from "~/components/shop/product/ProductCardSlide.vue";
import { useProductStyle } from "~/hooks";
import type { PropType } from "vue";

const props = defineProps({
    module: { type: Object, default: {} },
    productList: { type: Array as PropType<Array<{ productId: number; [k: string]: any }>>, default: () => [] },
    size: { type: Number, default: 1 },
    direction: { type: String, default: "" }
});

const productListRef = computed(() => props.productList ?? []);
const { styleMap } = useProductStyle(productListRef);
</script>

<style scoped lang="less">
.one-row-style {
    display: flex;

    width: 100%;
    box-sizing: border-box;
}
.can {
    flex-wrap: wrap;
}
.no {
    flex-wrap: nowrap;
    overflow-x: auto;
    /* 隐藏滚动条但保持滚动功能 */
    -ms-overflow-style: none; /* IE 和 Edge */
    scrollbar-width: none; /* Firefox */
    &::-webkit-scrollbar {
        display: none; /* Chrome, Safari, Opera*/
    }
}
</style>
