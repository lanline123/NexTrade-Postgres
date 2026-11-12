<template>
    <div v-if="picList.length" class="dec-pc-image-nav" :class="moduleClasses" :style="cssVars">
        <div class="dec-pc-image-nav__content">
            <div class="dec-pc-image-nav__grid" v-if="Number(moduleConfig.navType) === 1">
                <div v-for="(item, index) in fixedList" :key="index" class="dec-pc-image-nav__item">
                    <NuxtLink class="dec-pc-image-nav__link" target="_blank" :to="urlFormat(item.picLink)">
                        <div v-if="Number(moduleConfig.navStyle) === 1 || Number(moduleConfig.navStyle) === 2" class="dec-pc-image-nav__icon">
                            <img class="dec-pc-image-nav__image" :src="formatImage(item.picUrl)" :alt="item.picTitle" />
                        </div>
                        <div v-if="Number(moduleConfig.navStyle) === 1 || Number(moduleConfig.navStyle) === 3" class="dec-pc-image-nav__text">
                            {{ item.picTitle }}
                        </div>
                    </NuxtLink>
                </div>
            </div>

            <div v-else class="dec-pc-image-nav__carousel">
                <Swiper :key="swiperKey" :swiper-option="swiperOption" :item-list="swiperPages">
                    <template #default="{ item }">
                        <div class="dec-pc-image-nav__swiper-page">
                            <div v-for="(subItem, subIndex) in item" :key="subIndex" class="dec-pc-image-nav__item">
                                <NuxtLink
                                    v-if="!subItem.__isPlaceholder"
                                    class="dec-pc-image-nav__link"
                                    target="_blank"
                                    :to="urlFormat(subItem.picLink)"
                                >
                                    <div v-if="Number(moduleConfig.navStyle) === 1 || Number(moduleConfig.navStyle) === 2" class="dec-pc-image-nav__icon">
                                        <img class="dec-pc-image-nav__image" :src="formatImage(subItem.picUrl)" :alt="subItem.picTitle" />
                                    </div>
                                    <div v-if="Number(moduleConfig.navStyle) === 1 || Number(moduleConfig.navStyle) === 3" class="dec-pc-image-nav__text">
                                        {{ subItem.picTitle }}
                                    </div>
                                </NuxtLink>
                            </div>
                        </div>
                    </template>
                </Swiper>
            </div>
        </div>
    </div>
</template>

<script lang="ts" setup>
import { computed, ref } from "vue";
import Swiper from "~/decorate/components/swiper/Swiper.vue";
import { imageFormat, urlFormat } from "~/utils/format";

const props = defineProps({
    module: {
        type: Object,
        default: () => ({})
    }
});

const placeholderItem = ref({ __isPlaceholder: true });

const moduleConfig = computed(() => props.module || {});

const picList = computed(() => moduleConfig.value.picList || []);

const itemsPerPage = computed(() => {
    const rowNum = Number(moduleConfig.value.rowNum ?? 4);
    const colNum = Number(moduleConfig.value.colNum ?? 2);
    const safeRowNum = Number.isFinite(rowNum) ? Math.min(6, Math.max(3, rowNum)) : 4;
    const safeColNum = Number.isFinite(colNum) ? Math.min(3, Math.max(1, colNum)) : 2;
    return safeRowNum * safeColNum;
});

const fixedList = computed(() => picList.value.slice(0, itemsPerPage.value));

const swiperPages = computed(() => {
    if (Number(moduleConfig.value.navType) !== 2) return [];

    const pages: typeof picList.value[] = [];
    for (let i = 0; i < picList.value.length; i += itemsPerPage.value) {
        const page = picList.value.slice(i, i + itemsPerPage.value);
        while (page.length < itemsPerPage.value) {
            page.push(placeholderItem.value);
        }
        pages.push(page);
    }
    return pages;
});

const moduleClasses = computed(() => {
    const data = moduleConfig.value;
    const classes = [
        `dec-pc-image-nav--type-${data.navType ?? 1}`,
        `dec-pc-image-nav--style-${data.navStyle ?? 1}`,
        `dec-pc-image-nav--indicator-${data.picPageType ?? 1}`
    ];
    if (Number(data.radioStyle) === 2) {
        classes.push("dec-pc-image-nav--rounded");
    }
    return classes;
});

const cssVars = computed(() => {
    const data = moduleConfig.value;
    const frame = data.frame || {};
    // 内容背景色（内容设置）作用在内容区，容器背景色（扩展设置-容器设置）作用在外层，两者各自独立
    const contentBgColor = data.backgroundColor || "#ffffff";
    const outerBgColor = frame.backgroundColor || "#ffffff";
    const rowNum = Number(data.rowNum ?? 4);
    const colNum = Number(data.colNum ?? 2);

    return {
        "--dec-box-padding": `${frame.boxPadding ?? 10}px`,
        "--dec-box-padding-top": `${frame.boxPaddingTop ?? 5}px`,
        "--dec-box-padding-bottom": `${frame.boxPaddingBottom ?? 5}px`,
        "--dec-box-radius": `${frame.boxRadius ?? 0}px`,
        "--dec-bg-color": contentBgColor,
        "--dec-outer-bg": outerBgColor,
        "--dec-row-num": Number.isFinite(rowNum) ? Math.min(6, Math.max(3, rowNum)) : 4,
        "--dec-col-num": Number.isFinite(colNum) ? Math.min(3, Math.max(1, colNum)) : 2,
        "--dec-item-gap": `${data.imgPadding ?? 8}px`,
        "--dec-item-radius": Number(data.radioStyle) === 2 ? "8px" : "0",
        "--dec-indicator-color": data.swiperPageColor || "#333333"
    };
});

const swiperOption = computed(() => ({
    autoplay: {
        delay: 3000,
        disableOnInteraction: false,
        pauseOnMouseEnter: true
    },
    pagination: {
        clickable: true,
        enabled: true
    },
    loop: true,
    navigation: false
}));

const swiperKey = computed(() => `${swiperPages.value.length}-${itemsPerPage.value}-${moduleConfig.value.picPageType ?? 1}`);

function formatImage(url?: string): string {
    if (!url) return "";
    return imageFormat(url);
}
</script>

<style lang="less" scoped>
.dec-pc-image-nav {
    box-sizing: border-box;
    padding-left: var(--dec-box-padding, 10px);
    padding-right: var(--dec-box-padding, 10px);
    padding-top: var(--dec-box-padding-top, 5px);
    padding-bottom: var(--dec-box-padding-bottom, 5px);
    // 容器背景色（扩展设置 → 容器设置）作用在外层
    background-color: var(--dec-outer-bg, transparent);

    &__content {
        // 内容背景色（内容设置 → 背景颜色）作用在内容区
        background-color: var(--dec-bg-color, #ffffff);
        border-radius: var(--dec-box-radius, 0);
        overflow: hidden;
    }

    &__grid,
    &__swiper-page {
        display: grid;
        grid-template-columns: repeat(var(--dec-row-num, 4), minmax(0, 1fr));
        gap: var(--dec-item-gap, 8px);
        padding: var(--dec-item-gap, 8px);
        box-sizing: border-box;
    }

    &__item {
        min-width: 0;
    }

    &__link {
        display: flex;
        flex-direction: column;
        align-items: center;
        text-decoration: none;
        color: inherit;
        gap: 8px;
    }

    &__icon {
        width: 100%;
        display: flex;
        justify-content: center;
    }

    &__image {
        width: 100%;
        height: auto;
        display: block;
        border-radius: var(--dec-item-radius, 0);
        background: #f0f0f0;
    }

    &__text {
        font-size: 12px;
        color: #333;
        line-height: 1.2;
        text-align: center;
        word-break: break-all;
    }

    &__carousel :deep(.swiper) {
        padding-bottom: 30px;
    }

    :deep(.swiper-pagination-bullet-active) {
        background: var(--dec-indicator-color, #333333) !important;
    }
}

.dec-pc-image-nav--indicator-1 {
    :deep(.swiper-pagination-bullet) {
        width: 12px;
        height: 4px;
        border-radius: 999px;
        opacity: 0.35;
    }

    :deep(.swiper-pagination-bullet-active) {
        opacity: 1;
    }
}

.dec-pc-image-nav--indicator-2 {
    :deep(.swiper-pagination-bullet) {
        width: 6px;
        height: 6px;
        border-radius: 999px;
        opacity: 0.35;
    }

    :deep(.swiper-pagination-bullet-active) {
        opacity: 1;
    }
}
</style>
