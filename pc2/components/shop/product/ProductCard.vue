<template>
    <div :style="{ padding: module.goodsPadding + 'px', width: 100 / size + '%' }" class="product-card">
        <div
            class="car-info"
            :class="[
                module.goodsStyle == 2
                    ? 'product-card-border-2'
                    : module.goodsStyle == 3
                      ? 'product-card-border-3'
                      : module.goodsStyle == 1
                        ? ''
                        : 'product-card-border-4',
                module.goodsRadioStyle == 2 ? 'box-radius' : ''
            ]"
        >
            <div class="item-photo">
                <NuxtLink target="_blank" :to="productLink" class="item-content">
                    <el-image
                        :class="[module.goodsRadioStyle == 2 ? 'image-box-radius' : '']"
                        class="image-box"
                        :src="productImage(product)"
                        :alt="product.productName || ''"
                    >
                        <template #error>
                            <img class="image-box" src="@/assets/images/common/errorImg.png" :alt="product.productName || ''" />
                        </template>
                    </el-image>
                </NuxtLink>
                <img v-if="productStyle?.borderImage" class="product-style-border" :src="productStyle.borderImage" />
            </div>
            <div class="item-info product-info-Z2dxczIyODI" :style="{ padding: module.goodsNamePadding == 1 ? '0 10px' : '' }">
                <div class="item-name" :style="{ textAlign: module.textAlign == 1 ? 'left' : 'center' }">
                    <NuxtLink
                        v-if="module.showName"
                        target="_blank"
                        :to="productLink"
                        class="item-product-name"
                        :style="{ fontWeight: module.textWeight == 1 ? '' : 'bold', height: module.goodsNameRow == 1 ? '20px' : '40px' }"
                    >
                        <span v-if="product.deliveryMode === 2 && product.deliveryType === 1" class="cycleBuyTag">{{ $t("周期购") }}</span>
                        <span v-if="product.deliveryMode === 2 && product.deliveryType === 2" class="cycleBuyTag">{{ $t("次卡兑换") }}</span>
                        {{ product.productName }}
                    </NuxtLink>
                    <div v-if="module.showBrief && product.productBrief" class="item-brief">
                        {{ product.productBrief }}
                    </div>
                </div>
                <div class="item-action" :style="{ flexDirection: module.textAlign == 1 ? 'row' : 'column' }">
                    <div v-if="module.showPrice" class=" ">
                        <template v-if="product.deliveryMode === 2">
                            <FormatPrice
                                :fontStyle="{ color: 'var(--price)', fontSize: '24px', fontWeight: module.textWeight == 1 ? '' : 'bold' }"
                                v-model="product.price"
                            ></FormatPrice>
                        </template>
                        <template v-else>
                            <FormatPrice
                                :fontStyle="{ color: 'var(--price)', fontSize: '24px', fontWeight: module.textWeight == 1 ? '' : 'bold' }"
                                :model-value="cardPrice(product, product.productPrice)"
                            ></FormatPrice>
                        </template>
                    </div>
                    <div v-if="module.buyBtnStyle > 0" class="item-buy">
                        <template v-if="product.deliveryMode === 2">
                            <ProductBuy :isQuick="true" :shop-id="product.shopId" :id="product.productId" :disabled="product.productStock == 0" :number="1" :is-cycle-info="product.deliveryMode === 2">
                                <ProductButton :buyBtnStyle="module.buyBtnStyle"></ProductButton>
                            </ProductBuy>
                        </template>
                        <template v-else>
                            <template v-if="(product.productSku && product.productSku.length > 0) || [5, 6].includes(product.productType)">
                                <NuxtLink :to="productLink" target="_blank">
                                    <ProductButton :buyBtnStyle="module.buyBtnStyle"></ProductButton>
                                </NuxtLink>
                            </template>
                            <template v-else>
                                <ProductBuy :isQuick="true" :shop-id="product.shopId" :id="product.productId" :disabled="product.productStock == 0" :number="1">
                                    <ProductButton :buyBtnStyle="module.buyBtnStyle"></ProductButton>
                                </ProductBuy>
                            </template>
                        </template>
                    </div>
                </div>
            </div>
        </div>
    </div>
</template>
<script setup lang="ts">
import type { ProductActiveStyleVO } from "~/types/product/productStyle";
import ProductButton from "~/components/shop/product/ProductButton.vue";
import { productImage } from "~/utils/format";
import { useProductCardBuy } from "@/hooks/useProductCardBuy";

const { cardPrice, cardProductLink } = useProductCardBuy();

const props = defineProps({
    module: { type: Object, default: {} },
    product: { type: Object, default: {} },
    size: { type: Number, default: 1 },
    productStyle: { type: Object as () => ProductActiveStyleVO | null, default: null }
});

const productLink = computed(() =>
    cardProductLink(props.product, { path: "product", id: props.product.productId, sn: props.product.productSn })
);
</script>
<style scoped lang="less">
.product-card {
    display: flex;
    align-items: stretch;
    box-sizing: border-box;

    .car-info {
        padding-bottom: 10px;
        box-sizing: border-box;
        width: 100%;
        height: 100%;
        display: flex;
        flex-direction: column;
        background-color: white;

        .item-photo {
            position: relative;
            width: 100%;
            display: flex;

            .item-content {
                width: 100%;

                .image-box {
                    flex: 1;
                    width: 100%;
                }
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

        .item-info {
            display: flex;
            flex-direction: column;
            flex: 1;
            box-sizing: border-box;

            .item-name {
                display: flex;
                flex-direction: column;

                box-sizing: border-box;
                padding-bottom: 10px;

                .item-product-name {
                    font-size: 16px;
                    overflow: hidden; /* 隐藏超出容器的内容 */
                    text-overflow: ellipsis; /* 超出部分用省略号表示 */
                    line-height: 20px;
                }

                .item-brief {
                    margin-top: 4px;
                    line-height: 20px;
                    max-height: 40px;
                    color: #aaaaaa;
                    overflow: hidden;
                    display: -webkit-box;
                    -webkit-line-clamp: 2;
                    line-clamp: 2;
                    -webkit-box-orient: vertical;
                    white-space: normal;
                    word-break: break-all;
                }
            }

            .item-action {
                display: flex;
                align-items: center;
                justify-content: space-between;
                gap: 10px;
                margin-top: auto;

                .item-price {
                    font-size: 20px;
                    color: var(--general);
                    line-height: 30px;
                    height: 30px;
                }

                .item-buy {
                    display: flex;
                    justify-content: flex-end;
                }
            }
        }
    }

    .product-card-border-2 {
        box-shadow: 0 2px 4px rgba(0, 0, 0, 0.06);
    }

    .product-card-border-3 {
        border: 1px solid rgba(50, 50, 51, 0.1);
    }

    .product-card-border-4 {
        background-color: transparent;
    }
}

.box-radius {
    border-radius: 6px;
}

.image-box-radius {
    border-top-left-radius: 6px;
    border-top-right-radius: 6px;
}

.cycleBuyTag {
    color: var(--general);
    position: relative;
    margin-right: 8px;
    &::after {
        content: "";
        position: absolute;
        right: -5px;
        top: 50%;
        transform: translateY(-45%);
        width: 1px;
        height: 70%;
        background-color: var(--general);
    }
}
</style>
