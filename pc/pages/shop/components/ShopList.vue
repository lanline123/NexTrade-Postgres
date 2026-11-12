<template>
    <div class="shop-product-list">
        <div v-if="initialList && initialList.length > 0" class="nav-btn left-btn" :class="{ disabled: !canScrollLeft }" @click="scrollLeft">
            <tig-icon name="ArrowLeft" :size="12" />
        </div>

        <div ref="scrollContainer" class="swiper-wrapper" @scroll="handleScroll">
            <template v-if="initialList && initialList.length > 0">
                <div v-for="subItem in initialList" :key="subItem.productId" class="product-item">
                    <div class="product-img">
                        <NuxtLink :to="urlFormat({ path: 'product', seoLink: (subItem as any).seoLink, sn: subItem.productSn })" target="_blank" @click.stop="">
                            <img :src="productImage(subItem)" loading="lazy" :alt="subItem.productName || ''" />
                        </NuxtLink>
                    </div>
                    <div class="product-price">
                        <FormatPrice
                            v-model="subItem.productPrice"
                            class="pro_price"
                            :font-style="{ fontSize: '20px' }"
                            :currency-style="{ fontSize: '16px' }"
                        ></FormatPrice>
                    </div>
                </div>
            </template>
            <template v-else>
                <CommonAppEmpty :description="$t('暂无商品')" :image-size="60" />
            </template>
        </div>

        <div v-if="initialList && initialList.length > 0" class="nav-btn right-btn" :class="{ disabled: !canScrollRight }" @click="scrollRight">
            <tig-icon name="ArrowRight" :size="12" />
        </div>
    </div>
</template>

<script setup lang="ts">
// ===================== Vue 核心 =====================
import { ref, onMounted, computed } from "vue";

// ===================== 组件导入 =====================
import FormatPrice from "~/components/format/Price.vue";

// ===================== 类型导入 =====================
import type { shopListListingProduct } from "~/types/shop/shop.d";

// ===================== 工具函数 =====================
import { productImage, urlFormat } from "~/utils/format";

// ===================== Props & Emits =====================
defineProps<{
    initialList?: shopListListingProduct[];
}>();

// ===================== 响应式状态 =====================
const scrollContainer = ref<HTMLElement | null>(null);
const scrollPosition = ref(0);
const maxScroll = ref(0);

// ===================== 计算属性 =====================
const canScrollLeft = computed(() => scrollPosition.value > 0);
const canScrollRight = computed(() => scrollPosition.value < maxScroll.value - 5);

// ===================== 工具方法 =====================
const updateScrollState = (): void => {
    if (scrollContainer.value) {
        scrollPosition.value = scrollContainer.value.scrollLeft;
        maxScroll.value = scrollContainer.value.scrollWidth - scrollContainer.value.clientWidth;
    }
};

// ===================== 事件处理方法 =====================
const handleScroll = (): void => {
    updateScrollState();
};

const scrollLeft = (): void => {
    if (scrollContainer.value) {
        const scrollAmount = scrollContainer.value.clientWidth + 10;
        scrollContainer.value.scrollBy({ left: -scrollAmount, behavior: "smooth" });
    }
};

const scrollRight = (): void => {
    if (scrollContainer.value) {
        const scrollAmount = scrollContainer.value.clientWidth + 10;
        scrollContainer.value.scrollBy({ left: scrollAmount, behavior: "smooth" });
    }
};

// ===================== 生命周期 =====================
onMounted(() => {
    setTimeout(updateScrollState, 100);
});
</script>

<style scoped lang="less">
.shop-product-list {
    flex: 1;
    min-width: 0;
    position: relative;
    overflow: hidden;
    display: flex;
    align-items: flex-start;
    margin-left: 48px;

    .nav-btn {
        position: absolute;
        top: 72px;
        transform: translateY(-50%);
        width: 32px;
        height: 32px;
        background: #fff;
        border-radius: 50%;
        border: 1px solid #eee;
        display: flex;
        align-items: center;
        justify-content: center;
        cursor: pointer;
        z-index: 10;
        color: #111;
        font-weight: bold;
        transition: all 0.3s;
        user-select: none;

        &:hover {
            //background: #f5f5f5;
        }

        &.disabled {
            opacity: 0.5;
            cursor: not-allowed;
        }

        &.left-btn {
            left: 0;
        }

        &.right-btn {
            right: 0;
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
        min-height: 200px;
        align-items: flex-start;

        &::-webkit-scrollbar {
            display: none;
        }

        .product-item {
            width: 144px;
            flex-shrink: 0;
            display: flex;
            flex-direction: column;
            align-items: center;
            text-decoration: none;

            .product-img {
                width: 144px;
                height: 144px;
                overflow: hidden;
                margin-bottom: 12px;
                cursor: pointer;
                transition: opacity 0.3s;
                align-items: center;
                background-color: #f7f7f7;
                border-radius: 12px;
                display: flex;
                flex-direction: column;
                justify-content: flex-start;
                //position: relative;
                //z-index: 0;

                &:hover {
                    opacity: 0.9;
                }

                img {
                    width: 100%;
                    height: 100%;
                    object-fit: contain;
                    display: block;
                }
            }

            .product-price {
                width: 100%;
                text-align: center;

                .pro_price {
                    justify-content: center;
                    font-weight: 700;
                    color: var(--general);

                    :deep(.price) {
                        font-size: 28px;
                    }

                    :deep(.util),
                    :deep(.decimal) {
                        font-size: 20px;
                    }
                }
            }
        }
    }
}
</style>
