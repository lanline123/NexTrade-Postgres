<!--
  商品详情页加载骨架屏
  - 由 uiVersion 参数切换三套布局，分别对应 Product.vue 的 container-new / container / container-overseas，
    骨架块尺寸按各自真实布局对齐，尽量减少数据到位后的首屏跳动
  - 纯静态结构 + 纯 CSS 动画，不读取任何浏览器 API，SSR 与 CSR 首次渲染完全一致，可安全参与水合
-->
<template>
    <!-- ===================== 新版 UI 骨架屏 ===================== -->
    <div v-if="uiVersion === 'new'" class="product-skeleton product-skeleton--new" aria-hidden="true">
        <div class="topbar"></div>
        <div class="layout">
            <div class="head">
                <div class="sk sk--logo"></div>
                <div class="sk sk--search"></div>
            </div>
            <div class="body">
                <div class="col-left">
                    <div class="shop">
                        <div class="sk sk--shop-logo"></div>
                        <div class="shop-text">
                            <div class="sk sk--line sk--w40"></div>
                            <div class="sk sk--line sk--w60"></div>
                        </div>
                        <div class="shop-actions">
                            <div v-for="item in 2" :key="`shop-action-${item}`" class="sk sk--action"></div>
                        </div>
                    </div>

                    <div class="gallery">
                        <div class="thumbs">
                            <div v-for="item in 5" :key="`thumb-${item}`" class="sk sk--thumb"></div>
                        </div>
                        <div class="sk sk--main-pic"></div>
                    </div>

                    <div class="detail">
                        <div class="tabs">
                            <div v-for="item in 5" :key="`tab-${item}`" class="sk sk--tab"></div>
                        </div>
                        <div class="sk sk--line sk--title sk--w25"></div>
                        <div class="comments">
                            <div v-for="item in 2" :key="`comment-${item}`" class="comment">
                                <div class="comment-user">
                                    <div class="sk sk--avatar"></div>
                                    <div class="sk sk--line sk--w30"></div>
                                </div>
                                <div class="sk sk--line sk--w90"></div>
                                <div class="sk sk--line sk--w70"></div>
                            </div>
                        </div>
                    </div>
                </div>

                <div class="col-right">
                    <div class="sk sk--line sk--title sk--w95"></div>
                    <div class="sk sk--line sk--title sk--w60"></div>
                    <div class="sk sk--price-card"></div>
                    <div v-for="item in 2" :key="`sku-${item}`" class="sku">
                        <div class="sk sk--line sk--w20"></div>
                        <div class="sku-chips">
                            <div v-for="chip in 4" :key="`sku-${item}-chip-${chip}`" class="sk sk--chip"></div>
                        </div>
                    </div>
                    <div class="buttons">
                        <div v-for="item in 2" :key="`btn-${item}`" class="sk sk--button"></div>
                    </div>
                    <div class="sk sk--line sk--w50"></div>
                    <div class="sk sk--line sk--w80"></div>
                </div>
            </div>
        </div>
    </div>

    <!-- ===================== 跨境版 UI 骨架屏 ===================== -->
    <div v-else-if="uiVersion === 'overseas'" class="product-skeleton product-skeleton--overseas" aria-hidden="true">
        <div class="topbar"></div>
        <div class="classic-head">
            <div class="sk sk--logo"></div>
            <div class="sk sk--search"></div>
            <div class="sk sk--cart"></div>
        </div>
        <div class="layout">
            <div class="crumb-row">
                <div class="sk sk--line sk--w30"></div>
                <div class="share">
                    <div class="sk sk--line sk--share-label"></div>
                    <div v-for="item in 4" :key="`share-${item}`" class="sk sk--share-icon"></div>
                </div>
            </div>

            <div class="shop">
                <div class="sk sk--shop-logo"></div>
                <div class="shop-text">
                    <div class="sk sk--line sk--w30"></div>
                    <div class="sk sk--line sk--w50"></div>
                </div>
            </div>

            <div class="body">
                <div class="col-left">
                    <div class="gallery">
                        <div class="thumbs">
                            <div v-for="item in 5" :key="`thumb-${item}`" class="sk sk--thumb"></div>
                        </div>
                        <div class="sk sk--main-pic"></div>
                    </div>

                    <div class="tabs">
                        <div v-for="item in 5" :key="`tab-${item}`" class="sk sk--tab"></div>
                    </div>

                    <div class="desc">
                        <div class="sk sk--line sk--title sk--w20"></div>
                        <div v-for="item in 2" :key="`desc-${item}`" class="sk sk--desc-pic"></div>
                    </div>
                </div>

                <div class="col-right">
                    <div class="sk sk--line sk--title sk--w95"></div>
                    <div class="sk sk--line sk--title sk--w60"></div>
                    <div class="sk sk--price-card"></div>
                    <div v-for="item in 2" :key="`sku-${item}`" class="sku">
                        <div class="sk sk--line sk--w20"></div>
                        <div class="sku-chips">
                            <div v-for="chip in 4" :key="`sku-${item}-chip-${chip}`" class="sk sk--chip"></div>
                        </div>
                    </div>
                    <div class="buttons">
                        <div v-for="item in 2" :key="`btn-${item}`" class="sk sk--button"></div>
                    </div>
                    <div class="sk sk--line sk--w60"></div>
                </div>
            </div>
        </div>
    </div>

    <!-- ===================== 经典版 UI 骨架屏 ===================== -->
    <div v-else class="product-skeleton product-skeleton--old" aria-hidden="true">
        <div class="topbar"></div>
        <div class="classic-head">
            <div class="sk sk--logo"></div>
            <div class="sk sk--search"></div>
            <div class="sk sk--cart"></div>
        </div>
        <div class="layout">
            <div class="sk sk--line sk--crumb"></div>

            <div class="shop">
                <div class="sk sk--shop-logo"></div>
                <div class="shop-text">
                    <div class="sk sk--line sk--w30"></div>
                    <div class="shop-actions">
                        <div v-for="item in 2" :key="`shop-action-${item}`" class="sk sk--action"></div>
                    </div>
                </div>
            </div>

            <div class="body">
                <div class="col-left">
                    <div class="sk sk--main-pic"></div>
                    <div class="thumbs">
                        <div class="sk sk--thumb-arrow"></div>
                        <div v-for="item in 4" :key="`thumb-${item}`" class="sk sk--thumb"></div>
                        <div class="sk sk--thumb-arrow"></div>
                    </div>
                    <div class="share-wish">
                        <div class="sk sk--line sk--w50"></div>
                        <div class="sk sk--line sk--w20"></div>
                    </div>
                </div>

                <div class="col-right">
                    <div class="sk sk--line sk--title sk--w80"></div>
                    <div class="sk sk--line sk--w40"></div>
                    <div class="sk sk--price-card"></div>
                    <div class="sk sk--line sk--w60"></div>
                    <div v-for="item in 2" :key="`sku-${item}`" class="sku">
                        <div class="sk sk--line sk--w20"></div>
                        <div class="sku-chips">
                            <div v-for="chip in 4" :key="`sku-${item}-chip-${chip}`" class="sk sk--chip"></div>
                        </div>
                    </div>
                    <div class="qty">
                        <div class="sk sk--line sk--w20"></div>
                        <div class="sk sk--stepper"></div>
                    </div>
                    <div class="buttons">
                        <div v-for="item in 2" :key="`btn-${item}`" class="sk sk--button"></div>
                    </div>
                </div>
            </div>

            <div class="detail-main">
                <div class="side">
                    <div class="sk sk--side-card"></div>
                </div>
                <div class="main">
                    <div class="tabs">
                        <div v-for="item in 4" :key="`tab-${item}`" class="sk sk--tab"></div>
                    </div>
                    <div v-for="item in 2" :key="`desc-${item}`" class="sk sk--desc-pic"></div>
                </div>
            </div>
        </div>
    </div>
</template>

<script lang="ts" setup>
defineProps({
    // UI 版本：new 新版 / old 经典版 / overseas 跨境版，与 Product.vue 的 uiVersion 保持一致
    uiVersion: {
        type: String,
        default: "new"
    }
});
</script>

<style lang="less" scoped>
/* ===================== 骨架块通用样式 ===================== */
.product-skeleton {
    background: #fff;
    /* 撑满首屏，避免骨架屏比真实内容矮时页脚上蹿 */
    min-height: 100vh;
    padding-bottom: 60px;
    box-sizing: border-box;
    pointer-events: none;
    user-select: none;

    .topbar {
        height: 40px;
        background: #f5f6f8;
    }
}

.sk {
    position: relative;
    overflow: hidden;
    background: #f3f4f6;
    border-radius: 8px;

    &::after {
        content: "";
        position: absolute;
        inset: 0;
        transform: translateX(-100%);
        background: linear-gradient(90deg, rgba(255, 255, 255, 0) 0%, rgba(255, 255, 255, 0.72) 50%, rgba(255, 255, 255, 0) 100%);
        animation: product-skeleton-shimmer 1.4s ease-in-out infinite;
    }
}

.sk--line {
    height: 14px;
    border-radius: 999px;
}

.sk--title {
    height: 22px;
}

.sk--avatar {
    width: 32px;
    height: 32px;
    border-radius: 50%;
}

.sk--w20 {
    width: 20%;
}
.sk--w25 {
    width: 25%;
}
.sk--w30 {
    width: 30%;
}
.sk--w40 {
    width: 40%;
}
.sk--w50 {
    width: 50%;
}
.sk--w60 {
    width: 60%;
}
.sk--w70 {
    width: 70%;
}
.sk--w80 {
    width: 80%;
}
.sk--w90 {
    width: 90%;
}
.sk--w95 {
    width: 95%;
}

@keyframes product-skeleton-shimmer {
    100% {
        transform: translateX(100%);
    }
}

/* 经典版 / 跨境版共用的标准头部（CommonPageHeader）占位 */
.product-skeleton--old,
.product-skeleton--overseas {
    .classic-head {
        display: flex;
        align-items: center;
        gap: 40px;
        height: 100px;
        margin: 0 auto;
        box-sizing: border-box;

        .sk--logo {
            flex-shrink: 0;
            width: 190px;
            height: 50px;
        }

        .sk--search {
            flex: 1;
            min-width: 0;
            height: 40px;
        }

        .sk--cart {
            flex-shrink: 0;
            width: 110px;
            height: 40px;
        }
    }

    .shop-text {
        flex: 1;
        min-width: 0;
        margin-left: 12px;
        display: flex;
        align-items: center;
        justify-content: space-between;
        gap: 12px;
    }

    .sk--action {
        width: 93px;
        height: 32px;
    }

    .sku {
        display: flex;
        flex-direction: column;
        gap: 12px;
    }

    .sku-chips {
        display: flex;
        flex-wrap: wrap;
        gap: 10px;
    }

    .sk--chip {
        width: 88px;
        height: 32px;
    }
}

/* ===================== 新版 UI 骨架屏 ===================== */
.product-skeleton--new {
    .layout {
        max-width: 1760px;
        min-width: 1056px;
        width: 100%;
        margin: 0 auto;
        padding: 0 80px;
        box-sizing: border-box;
    }

    .head {
        height: 86px;
        padding-top: 16px;
        display: flex;
        align-items: center;
        justify-content: space-between;
        gap: 20px;

        .sk--logo {
            width: 135px;
            height: 40px;
        }

        .sk--search {
            width: 356px;
            height: 45px;
        }
    }

    .body {
        display: flex;
        justify-content: space-between;
        margin-top: 15px;
    }

    .col-left {
        min-width: 480px;
        width: calc(60% - 48px);
    }

    .col-right {
        flex-shrink: 0;
        margin-left: 48px;
        min-width: 400px;
        width: 40%;
        display: flex;
        flex-direction: column;
        gap: 16px;
    }

    .shop {
        display: flex;
        align-items: center;
        margin-bottom: 24px;

        .sk--shop-logo {
            flex-shrink: 0;
            width: 52px;
            height: 52px;
            border-radius: 12px;
        }
    }

    .shop-text {
        flex: 1;
        min-width: 0;
        margin-left: 12px;
        display: flex;
        flex-direction: column;
        gap: 8px;
    }

    .shop-actions {
        flex-shrink: 0;
        display: flex;
        gap: 12px;
        margin-left: 16px;

        .sk--action {
            width: 72px;
            height: 32px;
            border-radius: 999px;
        }
    }

    /* 对应 ThumbNew：左侧 72px 缩略图竖列 + 4:3 大图 */
    .gallery {
        display: flex;
        gap: 20px;
        margin-bottom: 10px;
    }

    .thumbs {
        flex-shrink: 0;
        width: 72px;
        display: flex;
        flex-direction: column;
        row-gap: 12px;

        .sk--thumb {
            width: 72px;
            height: 72px;
            border-radius: 12px;
        }
    }

    .sk--main-pic {
        flex: 1;
        min-width: 0;
        max-width: calc(620px * 4 / 3);
        aspect-ratio: 4 / 3;
        border-radius: 12px;
        background: #f5f5f5;
    }

    .detail {
        margin-top: 56px;
    }

    .tabs {
        display: flex;
        gap: 32px;
        padding-bottom: 28px;
        border-bottom: 1px solid #f0f0f0;
        margin-bottom: 28px;

        .sk--tab {
            width: 88px;
            height: 22px;
            border-radius: 999px;
        }
    }

    .comments {
        display: grid;
        gap: 24px;
        margin-top: 24px;
    }

    .comment {
        display: flex;
        flex-direction: column;
        gap: 12px;
    }

    .comment-user {
        display: flex;
        align-items: center;
        gap: 10px;
    }

    .sk--price-card {
        height: 104px;
        border-radius: 12px;
        margin: 8px 0;
    }

    .sku {
        display: flex;
        flex-direction: column;
        gap: 12px;
    }

    .sku-chips {
        display: flex;
        flex-wrap: wrap;
        gap: 12px;
    }

    .sk--chip {
        width: 96px;
        height: 36px;
    }

    .buttons {
        display: flex;
        gap: 16px;
        margin-top: 8px;

        .sk--button {
            flex: 1;
            height: 48px;
            border-radius: 999px;
        }
    }
}

/* ===================== 经典版 UI 骨架屏 ===================== */
.product-skeleton--old {
    .classic-head,
    .layout {
        width: 1190px;
        min-width: 1190px;
        margin: 0 auto;
    }

    .sk--crumb {
        width: 320px;
        margin-top: 16px;
    }

    .shop {
        display: flex;
        align-items: center;
        padding-top: 30px;

        .sk--shop-logo {
            flex-shrink: 0;
            width: 60px;
            height: 60px;
        }
    }

    .body {
        display: flex;
        justify-content: space-between;
        margin-top: 15px;
    }

    /* 对应 Thumb：358×358 大图 + 下方横排缩略图 */
    .col-left {
        width: 360px;
        flex-shrink: 0;
    }

    .sk--main-pic {
        width: 358px;
        height: 358px;
        border-radius: 0;
    }

    .thumbs {
        display: flex;
        align-items: center;
        gap: 10px;
        margin-top: 10px;

        .sk--thumb {
            width: 67.5px;
            height: 67.5px;
            border-radius: 0;
        }

        .sk--thumb-arrow {
            flex-shrink: 0;
            width: 18px;
            height: 70px;
            border-radius: 0;
        }
    }

    .share-wish {
        display: flex;
        align-items: center;
        gap: 10px;
        margin-top: 20px;
        padding-bottom: 30px;
    }

    .col-right {
        width: 807px;
        display: flex;
        flex-direction: column;
        gap: 14px;
    }

    .sk--price-card {
        height: 96px;
        border-radius: 4px;
    }

    .qty {
        display: flex;
        align-items: center;
        gap: 16px;

        .sk--stepper {
            flex-shrink: 0;
            width: 120px;
            height: 32px;
        }
    }

    .buttons {
        display: flex;
        gap: 16px;
        margin-top: 8px;

        .sk--button {
            width: 160px;
            height: 45px;
        }
    }

    .detail-main {
        display: flex;
        margin-bottom: 20px;
    }

    .side {
        width: 216px;
        flex-shrink: 0;

        .sk--side-card {
            width: 100%;
            height: 320px;
            border-radius: 0;
        }
    }

    .main {
        width: 964px;
        margin-left: 10px;

        .tabs {
            display: flex;
            gap: 24px;
            margin-bottom: 16px;

            .sk--tab {
                width: 96px;
                height: 34px;
                border-radius: 0;
            }
        }

        .sk--desc-pic {
            width: 100%;
            height: 320px;
            border-radius: 0;
            margin-bottom: 12px;
        }
    }
}

/* ===================== 跨境版 UI 骨架屏 ===================== */
.product-skeleton--overseas {
    .classic-head,
    .layout {
        width: 1264px;
        margin: 0 auto;
    }

    .crumb-row {
        display: flex;
        align-items: center;
        justify-content: space-between;
        column-gap: 60px;

        .sk--line {
            max-width: 320px;
        }
    }

    .share {
        display: flex;
        align-items: center;
        gap: 16px;
        min-width: 196px;
        flex-shrink: 0;

        .sk--share-label {
            width: 56px;
        }

        .sk--share-icon {
            width: 18px;
            height: 18px;
            border-radius: 50%;
        }
    }

    .shop {
        display: flex;
        align-items: center;
        padding-top: 20px;

        .sk--shop-logo {
            flex-shrink: 0;
            width: 60px;
            height: 60px;
        }
    }

    .body {
        display: flex;
        justify-content: space-between;
        margin-top: 20px;
    }

    /* 对应 overseas/Thumb：左侧 112px 缩略图竖列 + 624×624 大图 */
    .col-left {
        width: 752px;
        flex-shrink: 0;
    }

    .gallery {
        display: flex;
        margin-bottom: 16px;
    }

    .thumbs {
        flex-shrink: 0;
        width: 112px;
        display: flex;
        flex-direction: column;
        row-gap: 12px;

        .sk--thumb {
            width: 104px;
            height: 104px;
            border-radius: 0;
        }
    }

    .sk--main-pic {
        width: 624px;
        height: 624px;
        border-radius: 0;
    }

    .tabs {
        display: flex;
        column-gap: 35px;
        padding: 16px 0;
        margin-bottom: 16px;

        .sk--tab {
            width: 64px;
            height: 22px;
        }
    }

    .desc {
        width: 738px;

        .sk--title {
            margin-bottom: 24px;
        }

        .sk--desc-pic {
            width: 100%;
            height: 360px;
            border-radius: 0;
            margin-bottom: 12px;
        }
    }

    .col-right {
        width: 496px;
        flex-shrink: 0;
        margin-left: 16px;
        padding: 0 16px;
        box-sizing: border-box;
        display: flex;
        flex-direction: column;
        gap: 16px;
    }

    .sk--price-card {
        height: 96px;
        border-radius: 4px;
    }

    .buttons {
        display: flex;
        flex-direction: column;
        gap: 12px;
        margin-top: 8px;

        .sk--button {
            width: 100%;
            height: 48px;
            border-radius: 999px;
        }
    }
}
</style>
