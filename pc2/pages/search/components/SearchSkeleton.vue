<template>
    <div class="search-page-skeleton" aria-hidden="true">
        <div class="search-page-skeleton__category-row">
            <div v-for="item in 7" :key="`category-${item}`" class="search-page-skeleton__category-card">
                <div class="search-page-skeleton__category-image"></div>
                <div class="search-page-skeleton__category-text"></div>
            </div>
        </div>
        <div class="search-page-skeleton__tabs-row">
            <div class="search-page-skeleton__tabs-left">
                <div class="search-page-skeleton__tab"></div>
                <div class="search-page-skeleton__tab search-page-skeleton__tab--short"></div>
            </div>
            <div class="search-page-skeleton__pager"></div>
        </div>
        <div class="search-page-skeleton__sort-row">
            <div v-for="item in 6" :key="`sort-${item}`" class="search-page-skeleton__sort-chip"></div>
        </div>
        <div class="search-page-skeleton__goods-grid">
            <div v-for="item in 8" :key="`goods-${item}`" class="search-page-skeleton__goods-card">
                <div class="search-page-skeleton__goods-image"></div>
                <div class="search-page-skeleton__goods-line search-page-skeleton__goods-line--title"></div>
                <div class="search-page-skeleton__goods-line"></div>
                <div class="search-page-skeleton__goods-line search-page-skeleton__goods-line--short"></div>
                <div class="search-page-skeleton__goods-footer">
                    <div class="search-page-skeleton__price"></div>
                    <div class="search-page-skeleton__button"></div>
                </div>
            </div>
        </div>
    </div>
</template>

<script lang="ts" setup>
/**
 * 商品搜索页首屏骨架屏
 * 商品搜索页和图片搜索结果页的首次加载共用；定位由使用方决定
 */
</script>

<style lang="less" scoped>
.search-page-skeleton__category-row,
.search-page-skeleton__tabs-row,
.search-page-skeleton__sort-row,
.search-page-skeleton__goods-grid {
    display: grid;
}

.search-page-skeleton__category-row {
    grid-template-columns: repeat(7, minmax(0, 1fr));
    gap: 8px;
    padding: 16px 0 24px;
}

.search-page-skeleton__category-card,
.search-page-skeleton__tab,
.search-page-skeleton__pager,
.search-page-skeleton__sort-chip,
.search-page-skeleton__goods-card,
.search-page-skeleton__category-image,
.search-page-skeleton__category-text,
.search-page-skeleton__goods-image,
.search-page-skeleton__goods-line,
.search-page-skeleton__price,
.search-page-skeleton__button {
    position: relative;
    overflow: hidden;
    background: #f3f4f6;
}

.search-page-skeleton__category-card,
.search-page-skeleton__goods-card {
    background: #fff;
    border: 1px solid #f0f0f0;
    border-radius: 12px;
}

.search-page-skeleton__category-card::after,
.search-page-skeleton__tab::after,
.search-page-skeleton__pager::after,
.search-page-skeleton__sort-chip::after,
.search-page-skeleton__goods-card::after,
.search-page-skeleton__category-image::after,
.search-page-skeleton__category-text::after,
.search-page-skeleton__goods-image::after,
.search-page-skeleton__goods-line::after,
.search-page-skeleton__price::after,
.search-page-skeleton__button::after {
    content: "";
    position: absolute;
    inset: 0;
    transform: translateX(-100%);
    background: linear-gradient(90deg, rgba(255, 255, 255, 0) 0%, rgba(255, 255, 255, 0.72) 50%, rgba(255, 255, 255, 0) 100%);
    animation: search-page-skeleton-shimmer 1.4s ease-in-out infinite;
}

.search-page-skeleton__category-card {
    display: flex;
    align-items: center;
    gap: 10px;
    height: 48px;
    padding: 4px 12px 4px 4px;
}

.search-page-skeleton__category-image {
    width: 40px;
    height: 40px;
    border-radius: 8px;
    flex-shrink: 0;
}

.search-page-skeleton__category-text {
    width: 70%;
    height: 14px;
    border-radius: 999px;
}

.search-page-skeleton__tabs-row {
    grid-template-columns: 1fr auto;
    align-items: center;
    gap: 24px;
    padding-bottom: 20px;
}

.search-page-skeleton__tabs-left {
    display: flex;
    align-items: center;
    gap: 32px;
}

.search-page-skeleton__tab {
    width: 92px;
    height: 24px;
    border-radius: 999px;
}

.search-page-skeleton__tab--short {
    width: 64px;
}

.search-page-skeleton__pager {
    width: 110px;
    height: 24px;
    border-radius: 999px;
}

.search-page-skeleton__sort-row {
    grid-template-columns: repeat(6, 112px);
    gap: 10px;
    padding-bottom: 24px;
}

.search-page-skeleton__sort-chip {
    height: 32px;
    border-radius: 8px;
}

.search-page-skeleton__goods-grid {
    grid-template-columns: repeat(var(--search-grid-columns, 5), minmax(0, 1fr));
    gap: 16px;
}

.search-page-skeleton__goods-card {
    padding: 12px;
    border-radius: var(--search-item-radius, 12px);
}

.search-page-skeleton__goods-image {
    width: 100%;
    aspect-ratio: 1 / 1;
    border-radius: inherit;
}

.search-page-skeleton__goods-line {
    height: 14px;
    border-radius: 999px;
    margin-top: 12px;
}

.search-page-skeleton__goods-line--title {
    width: 82%;
    margin-top: 16px;
}

.search-page-skeleton__goods-line--short {
    width: 56%;
}

.search-page-skeleton__goods-footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    margin-top: 16px;
}

.search-page-skeleton__price {
    width: 88px;
    height: 20px;
    border-radius: 999px;
}

.search-page-skeleton__button {
    width: 72px;
    height: 32px;
    border-radius: 999px;
}

@keyframes search-page-skeleton-shimmer {
    100% {
        transform: translateX(100%);
    }
}
</style>
