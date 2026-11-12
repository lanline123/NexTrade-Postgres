<template>
    <div class="mod_you_like">
        <h3 class="com-pc-title" v-if="titleName">{{ $t(titleName) }}</h3>
        <div ref="listRef" class="you_like_list" :style="{ '--you-like-columns': columns }">
            <template v-for="item in displayList" :key="item.productId">
                <ProductCard :product="item" @callback="$emit('callback')"></ProductCard>
            </template>
        </div>
        <div ref="recommendMoreRef">
            <div v-if="isLoading" class="loading-con">
                <div class="gif-loading"></div>
            </div>
            <div v-if="isloadEnd" class="load-end-con">{{ $t("没有更多喽") }}~</div>
        </div>
    </div>
</template>
<script lang="ts" setup>
/**
 * 猜你喜欢商品轮播组件
 * - 根据用户浏览记录获取推荐商品ID
 * - 支持滚动加载更多（无限滚动）
 * - 用于购物车页面底部推荐
 */
import { onMounted, onUnmounted, ref, computed, watch } from "vue";
import { getProductsList } from "@/api/common";
import type { ProductsListResult } from "@/types/common/common";
import ProductCard from "@/components/product/ProductCard.vue";
import { getProductLikeListIds } from "~/api/product/product";
import { useResponsiveGridColumns } from "~/hooks/useResponsiveGridColumns";
defineProps({
    titleName: String
});

const emit = defineEmits(["callback"]);

// 固定每页数量以保证 page 分页稳定；每行铺满交由 columns 与 displayList 控制
const PAGE_SIZE = 12;

// cart 容器锁定 min-width 1264（内容区最窄约 1188），下限取 5 列避免出现过宽的 4 列，与搜索页密度一致
const { containerRef: listRef, columns } = useResponsiveGridColumns({ minCardWidth: 240, gap: 16, minColumns: 5, maxColumns: 6 });

const isLoading = ref(true);
const isloadEnd = ref(false);
const page = ref(1);
const total = ref(0);
const productList = ref<ProductsListResult[]>([]);
const isNoMore = ref(false);
let requestGeneration = 0;

// 已到数据尽头：接口返回不足一页，或已取满 total
const reachedEnd = computed(() => isNoMore.value || (total.value > 0 && productList.value.length >= total.value));

// 未到尽头时仅渲染能铺满整行的部分，余数留到下一页补满再显示，避免末行残缺
const displayList = computed(() => {
    if (reachedEnd.value) return productList.value;
    const fullCount = Math.floor(productList.value.length / columns.value) * columns.value;
    return productList.value.slice(0, fullCount);
});

/** 获取推荐商品列表，先获取推荐ID再查询商品详情 */
const getProductsListData = async () => {
    const currentRequest = ++requestGeneration;
    isLoading.value = true;
    try {
        const ids = await getProductLikeListIds();
        const result = await getProductsList({ page: page.value, size: PAGE_SIZE, ids });
        if (currentRequest !== requestGeneration) return;
        total.value = result.total;
        productList.value = [...productList.value, ...result.records];
        if (result.records.length < PAGE_SIZE) {
            isNoMore.value = true;
        }
    } catch (error) {
        console.log(error);
    } finally {
        if (currentRequest !== requestGeneration) return;
        isLoading.value = false;
        isloadEnd.value = reachedEnd.value;
    }
};


const recommendMoreRef = ref<HTMLElement | null>(null);

const handleScroll = () => {
    const container = recommendMoreRef.value;
    if (!container) return;

    const containerTop = container.offsetTop;
    const containerHeight = container.offsetHeight;
    const windowHeight = window.innerHeight;
    const scrollY = window.scrollY;
    // 检查是否滚动到指定容器
    if (scrollY + windowHeight > containerTop + containerHeight && productList.value.length < total.value && !isLoading.value && !isNoMore.value) {
        page.value++;
        getProductsListData();
    }
};

// 监听滚动事件
onMounted(() => {
    window.addEventListener("scroll", handleScroll);
    getProductsListData();
});

// 移除滚动事件监听
onUnmounted(() => {
    window.removeEventListener("scroll", handleScroll);
});
</script>
<style lang="less" scoped>
.com-pc-title {
    position: relative;
    width: fit-content;
    // max-width: 255px;
    height: 46px;
    white-space: nowrap;
    font-size: 24px;
    font-weight: 700;
    color: #1a1a1a;
    text-align: left;
    padding-bottom: 4px;
    overflow: hidden;
    color: #333;
    display: flex;
    align-items: flex-start;
    justify-content: space-around;
}

.loading-con {
    display: flex;
    justify-content: center;
}

.load-end-con {
    text-align: center;
    margin-bottom: 20px;
}

.com-pc-title:before {
    content: "";
    position: absolute;
    width: 36px;
    left: 50%;
    bottom: 0;
    height: 3px;
    border-radius: 1px;
    background-color: #1a1a1a;
    transition:
        left 0.3s ease,
        width 0.3s ease;
    transform: translateX(-50%);
}

.mod_you_like {
    margin-top: 91px;
    width: 100%;
}

.mod_you_like .you_like_list {
    margin-top: 32px;
    position: relative;
    padding-bottom: 50px;
    display: grid;
    grid-template-columns: repeat(var(--you-like-columns, 5), minmax(0, 1fr));
    gap: 16px;

    // 卡片填满弹性列宽、图片随列宽保持正方形（覆盖 ProductCard 的固定 240px）
    :deep(.product-card-container) {
        width: 100%;
    }
    :deep(.product-card-container .pro_pic) {
        width: 100%;
        height: auto;
        aspect-ratio: 1 / 1;
    }
}
</style>
