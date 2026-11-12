<template>
    <div class="product-item">
        <!-- 修复双图重复渲染，样式边框图移出链接 -->
        <div class="product-img">
            <NuxtLink :to="urlFormat({ path: 'product', seoLink: product.seoLink, id: product.productId, sn: product.productSn })" target="_blank">
                <el-image class="img" lazy :src="productImage(product)" :alt="product.productName || ''" />
            </NuxtLink>
            <img v-if="productStyle?.borderImage" class="product-style-border" :src="productStyle.borderImage || undefined" alt="" />
        </div>
        <div class="product-info">
            <NuxtLink :to="urlFormat({ path: 'product', seoLink: product.seoLink, id: product.productId, sn: product.productSn })" target="_blank">
                <div class="product-name line1">{{ product.productName }}</div>
            </NuxtLink>
            <div v-if="skuDesc" class="product-sku-box">
                <div class="product-sku line1">{{ skuDesc }}</div>
            </div>
            <div class="product-price">
                <format-price
                    :style="{ fontSize: '36rpx', color: '#A9781B' }"
                    :currency-style="{
                        fontSize: '24rpx'
                    }"
                    :decimals-style="{
                        fontSize: '24rpx'
                    }"
                    :price-data="product.skuId ? product.skuPrice : product.productPrice"
                />
                <div class="product-num">x1</div>
            </div>
        </div>
    </div>
</template>

<script lang="ts" setup>
/**
 * @description 礼品卡可兑换商品项组件
 * - 展示商品图片、名称、规格、价格
 * - 点击可跳转到商品详情页
 */
import type { GiftCardProduct } from "~/types/giftCard/giftCard";
import type { ProductActiveStyleVO } from "~/types/product/productStyle";

const props = defineProps<{
    product: GiftCardProduct;
    productStyle?: ProductActiveStyleVO | null;
}>();

const skuDesc = computed(() => {
    if (!props.product.skuData || !props.product.skuData.length) {
        return "";
    }
    try {
        const skuInfo = JSON.parse(props.product.skuData);
        return skuInfo
            .map((item: any) => {
                return item.name + ": " + item.value;
            })
            .join(" ");
    } catch {
        return "";
    }
});
</script>

<style lang="less" scoped>
.product-item {
    display: flex;
    gap: 15px;
    .product-img {
        flex-shrink: 0;
        border-radius: 4px;
        width: 75px;
        height: 75px;
        overflow: hidden;
        background: #f2f2f2;
        position: relative;
        .img {
            text-align: center;
            display: flex;
            justify-content: center;
            width: 100%;
            height: 100%;
        }

        .product-style-border {
            position: absolute;
            left: 0;
            top: 0;
            width: 100%;
            height: 100%;
            z-index: 5;
            pointer-events: none;
            object-fit: fill;
        }
    }
    .product-info {
        flex: 1;
        display: flex;
        flex-direction: column;
        justify-content: space-between;
        min-width: 0;
        .product-name {
            display: -webkit-box;
            -webkit-box-orient: vertical;
            -webkit-line-clamp: 1;
            font-size: 16px;
            color: #262626;
        }
        .product-sku-box {
            display: flex;
            .product-sku {
                font-size: 12px;
                line-height: 12px;
                color: #8c8c8c;
                padding: 5px 16px;
                background: #f5f5f5;
                border-radius: 11px;
                overflow: hidden;
                flex-grow: 0;
                max-width: 80%;
            }
        }
    }
    .product-price {
        display: flex;
        align-items: flex-end;
        justify-content: space-between;
        .product-num {
            font-size: 14px;
            color: #262626;
            font-weight: bold;
        }
    }
}
</style>
