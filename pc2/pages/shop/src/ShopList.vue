<template>
    <div class="shop-product-list">
        <div class="nav-btn left-btn" :class="{ disabled: !canScrollLeft }" @click="scrollLeft" v-if="initialList && initialList.length > 0">
            <el-icon><ArrowLeft /></el-icon>
        </div>

        <div class="swiper-wrapper" ref="scrollContainer" @scroll="handleScroll">
            <template v-if="initialList && initialList.length > 0">
                <div class="product-item" v-for="subItem in initialList" :key="subItem.productId">
                    <div class="product-img">
                        <NuxtLink @click.stop="" :to="urlFormat({ path: 'product', seoLink: subItem.seoLink, sn: subItem.productSn })" target="_blank">
                            <el-image :src="productImage(subItem)" lazy fit="contain" :alt="subItem.productName">
                                <template #placeholder>
                                    <div class="image-slot loading-slot">
                                        <span class="dot">...</span>
                                    </div>
                                </template>
                            </el-image>
                        </NuxtLink>
                        <img
                            v-if="styleMap[subItem.productId]?.borderImage"
                            class="product-style-border"
                            :src="styleMap[subItem.productId].borderImage || undefined"
                            alt=""
                        />
                    </div>
                    <div class="product-price">
                        <FormatPrice v-model="subItem.productPrice" class="pro_price"></FormatPrice>
                    </div>
                </div>
            </template>
            <template v-else>
                <el-empty :description="$t('暂无商品')" :image-size="60" />
            </template>
        </div>

        <div class="nav-btn right-btn" :class="{ disabled: !canScrollRight }" @click="scrollRight" v-if="initialList && initialList.length > 0">
            <el-icon><ArrowRight /></el-icon>
        </div>
    </div>
</template>

<script setup lang="ts">
/**
 * @description 店铺商品列表组件（店铺街行卡片布局）
 * - 横向滚动展示店铺热销商品，支持左右箭头滚动控制
 * - 叠加商品样式边框（useProductStyle）
 */
import { ref, onMounted, computed } from "vue";
import { productImage, urlFormat } from "@/utils/format";
import FormatPrice from "@/components/format/Price.vue";
import { ArrowLeft, ArrowRight } from "@element-plus/icons-vue";
import { useProductStyle } from "~/hooks";
import type { shopListListingProduct } from "~/types/shop/shop.d";

const props = defineProps<{
    initialList?: shopListListingProduct[];
}>();

const initialListRef = computed(() => props.initialList || []);
const { styleMap } = useProductStyle(initialListRef);

const scrollContainer = ref<HTMLElement | null>(null);
const scrollPosition = ref(0);
const maxScroll = ref(0);

const canScrollLeft = computed(() => scrollPosition.value > 0);
const canScrollRight = computed(() => scrollPosition.value < maxScroll.value - 5);

const updateScrollState = () => {
    if (scrollContainer.value) {
        scrollPosition.value = scrollContainer.value.scrollLeft;
        maxScroll.value = scrollContainer.value.scrollWidth - scrollContainer.value.clientWidth;
    }
};

const handleScroll = () => {
    updateScrollState();
};

const scrollLeft = () => {
    if (scrollContainer.value) {
        const scrollAmount = scrollContainer.value.clientWidth + 10;
        scrollContainer.value.scrollBy({ left: -scrollAmount, behavior: "smooth" });
    }
};

const scrollRight = () => {
    if (scrollContainer.value) {
        const scrollAmount = scrollContainer.value.clientWidth + 10;
        scrollContainer.value.scrollBy({ left: scrollAmount, behavior: "smooth" });
    }
};

onMounted(() => {
    setTimeout(updateScrollState, 100);
});
</script>

<style scoped lang="less">
.shop-product-list {
    flex: 1;
    position: relative;
    overflow: hidden;
    padding: 10px;
    display: flex;
    align-items: center;

    .nav-btn {
        position: absolute;
        top: 50%;
        transform: translateY(-50%);
        width: 24px;
        height: 24px;
        background: #fff;
        border-radius: 50%;
        border: 1px solid #eee;
        display: flex;
        align-items: center;
        justify-content: center;
        cursor: pointer;
        z-index: 10;
        color: #666;
        font-weight: bold;
        transition: all 0.3s;
        user-select: none;

        &:hover {
            background: #f5f5f5;
            color: var(--general);
            border-color: var(--general);
        }

        &.disabled {
            opacity: 0.5;
            cursor: not-allowed;
        }

        &.left-btn {
            left: 12px;
        }

        &.right-btn {
            right: 12px;
        }

        .el-icon {
            font-size: 12px;
        }
    }

    .swiper-wrapper {
        display: flex;
        flex-wrap: nowrap;
        overflow-x: auto;
        scrollbar-width: none;
        -ms-overflow-style: none;
        width: 100%;
        gap: 16px;
        scroll-behavior: smooth;
        min-height: 180px;
        align-items: flex-start;

        &::-webkit-scrollbar {
            display: none;
        }

        .product-item {
            width: 140px;
            flex-shrink: 0;
            display: flex;
            flex-direction: column;
            align-items: center;

            .product-img {
                width: 140px;
                height: 140px;
                border-radius: 8px;
                overflow: hidden;
                border: 1px solid #f5f5f5;
                background: #fff;
                margin-bottom: 8px;
                cursor: pointer;
                transition: opacity 0.3s;
                position: relative;

                &:hover {
                    opacity: 0.9;
                }

                :deep(.el-image) {
                    width: 100%;
                    height: 100%;
                    object-fit: contain;
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

            .product-price {
                width: 100%;
                text-align: center;

                .pro_price {
                    justify-content: center;
                    font-size: 14px;
                    font-weight: bold;
                    color: #e23e3e;

                    :deep(.num) {
                        font-size: 16px;
                    }
                }
            }
        }
    }
}
</style>
