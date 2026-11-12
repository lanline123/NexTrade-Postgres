<template>
    <div class="product-list" v-if="productList && productList.length > 0">
        <div v-for="(product, index) in productList" :key="index" class="product-item">
            <div class="product-content" @click="handleLink(product)">
                <div class="product-left">
                    <el-image
                        class="product-img"
                        :src="productImage(product)"
                        fit="cover"
                        loading="lazy"
                        :alt="product.productName || ''"
                    >
                        <template #error>
                            <img class="product-img" src="@/assets/images/common/errorImg.png" :alt="product.productName || ''" />
                        </template>
                    </el-image>
                    <img
                        v-if="product.productId && styleMap[product.productId]?.borderImage"
                        class="product-style-border"
                        :src="styleMap[product.productId].borderImage"
                    />
                </div>
                <div class="product-right">
                    <div class="product-title line2">{{ product.productName }}</div>
                    <div class="product-price-box">
                        <div class="product-price">
                            <FormatPrice v-model="product.productPrice" :style="{ fontSize: '14px' }" :decimalsStyle="{ fontSize: '12px', alignSelf: 'flex-end' }"></FormatPrice>
                            <FormatPrice
                                v-if="product.marketPrice && product.marketPrice !== product.productPrice"
                                v-model="product.marketPrice"
                                :style="{ textDecoration: 'line-through', color: '#969799', marginLeft: '10px' }"
                            ></FormatPrice>
                        </div>
                        <div class="product-num">x{{ product.quantity || 1 }}</div>
                    </div>
                </div>
            </div>
        </div>
    </div>
</template>

<script setup lang="ts">
/**
 * 文章关联商品展示组件
 * - 根据传入的商品 ID 列表批量获取商品详情
 * - 支持两种数据格式：纯 ID 数组或包含 id 和 quantity 的对象数组
 * - 点击商品卡片跳转到商品详情页（新窗口打开）
 * - 展示商品图片、名称、价格、划线价、数量
 */
import { ref, watch, onMounted } from "vue";
import { useProductStyle } from "~/hooks";
import { getProductDetail } from "@/api/product/product";
import type { ProductItem } from "@/types/product/product.d";
import { productImage, urlFormat } from "@/utils/format";

interface ProductWithQuantity extends ProductItem {
    quantity?: number;
}

const props = defineProps({
    productIds: {
        type: Array,
        default: () => []
    }
});

const productList = ref<ProductWithQuantity[]>([]);
const { styleMap } = useProductStyle(productList);
const loading = ref(false);

// 获取商品详情
const fetchProductDetails = async () => {
    if (!props.productIds || props.productIds.length === 0) {
        productList.value = [];
        return;
    }

    loading.value = true;
    try {
        const promises = props.productIds.map(async (item: any) => {
            // 如果 productIds 是对象数组，包含 id 和 quantity
            if (typeof item === "object" && item.id) {
                try {
                    const result = await getProductDetail({ id: item.id });
                    return {
                        ...result.item,
                        quantity: item.quantity || 1
                    };
                } catch (error) {
                    console.error(`获取商品 ${item.id} 详情失败:`, error);
                    return null;
                }
            }
            // 如果 productIds 只是 ID 数组
            else if (typeof item === "number" || typeof item === "string") {
                try {
                    const result = await getProductDetail({ id: Number(item) });
                    return {
                        ...result.item,
                        quantity: 1
                    };
                } catch (error) {
                    console.error(`获取商品 ${item} 详情失败:`, error);
                    return null;
                }
            }
            return null;
        });

        const results = await Promise.all(promises);
        productList.value = results.filter((item) => item !== null) as ProductWithQuantity[];
    } catch (error) {
        console.error("获取商品列表失败:", error);
    } finally {
        loading.value = false;
    }
};

// 跳转到商品详情页
const handleLink = (product: ProductWithQuantity) => {
    const url = urlFormat({ path: "product", seoLink: product.seoLink, id: product.productId, sn: product.productSn });
    if (!url) return;
    window.open(url, "_blank");
};

// 监听 productIds 变化
watch(
    () => props.productIds,
    () => {
        fetchProductDetails();
    },
    { deep: true, immediate: true }
);

onMounted(() => {
    fetchProductDetails();
});
</script>

<style scoped lang="less">
.product-list {
    margin-top: 20px;
    display: flex;
    flex-wrap: wrap;
    gap: 10px;
}

.product-item {
    flex: 0 0 calc(50% - 5px);
    padding: 10px;
    border: 1px solid #e6e6e6;
    border-radius: 4px;
    box-sizing: border-box;
    background-color: #fff;
    transition: box-shadow 0.2s;

    &:hover {
        // box-shadow: 0 2px 12px rgba(0, 0, 0, 0.1);
    }

    .product-content {
        display: flex;
        cursor: pointer;
        transition: opacity 0.2s;

        &:hover {
            opacity: 0.8;
        }
    }

    .product-left {
        width: 80px;
        height: 80px;
        overflow: hidden;
        flex-shrink: 0;
        position: relative;

        .product-img {
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

    .product-right {
        flex: 1;
        padding-left: 16px;
        display: flex;
        flex-direction: column;
        justify-content: space-between;

        .product-title {
            font-size: 14px;
            color: #333;
            font-weight: normal;
            line-height: 1.5;
            margin-bottom: 10px;
        }

        .product-price-box {
            display: flex;
            justify-content: space-between;
            align-items: center;
        }

        .product-price {
            display: flex;
            align-items: center;
            flex: 1;
        }

        .product-num {
            font-size: 14px;
            color: #969799;
            margin-left: 20px;
        }
    }
}

.line2 {
    display: -webkit-box;
    -webkit-box-orient: vertical;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    overflow: hidden;
    text-overflow: ellipsis;
}
</style>
