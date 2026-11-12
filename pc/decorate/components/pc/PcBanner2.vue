<template>
    <div v-if="picList.length" class="dec-banner2" :class="moduleClasses" :style="cssVars">
        <div class="dec-banner2__inner">
            <Swiper :key="swiperKey" :swiper-option="swiperOption" :item-list="picList">
                <template #default="{ item }">
                    <div class="dec-banner2__item">
                        <NuxtLink
                            class="dec-banner2__link"
                            target="_blank"
                            :to="urlFormat(item.picLink)"
                            :title="getAltText(item)"
                            :aria-label="getAltText(item)"
                        >
                            <img class="dec-banner2__image" loading="lazy" :src="formatImage(item.picUrl)" :alt="getAltText(item)" />
                        </NuxtLink>
                    </div>
                </template>
            </Swiper>
        </div>
    </div>
</template>

<script setup lang="ts">
/**
 * PC 轮播 Banner2（新套）
 * 对齐 Admin Preview / vortmall PcBanner2Wrapper：
 * - 外边距落在外层（不被背景色吞掉）
 * - 背景色 / 圆角 / 左右内边距落在内层
 * - 高度跟随图片比例，不再写死 280
 * - slideGap → swiper spaceBetween
 */
import { computed, type PropType } from "vue";
import Swiper from "~/decorate/components/swiper/Swiper.vue";
import { imageFormat, urlFormat } from "~/utils/format";
import { truncateSeoText } from "~/utils/seo";
import { useCommonStore } from "~/store/common";
import type { DecorateModulePayload } from "~/types/decoration/decorateModule";

const props = defineProps({
    module: { type: Object as PropType<DecorateModulePayload>, default: () => ({}) }
});

const commonStore = useCommonStore() as any;
const siteName = computed(() => commonStore.shopTitle || commonStore.shopName || "");

const moduleConfig = computed<Record<string, any>>(() => (props.module || {}) as Record<string, any>);
const frame = computed<Record<string, any>>(() => moduleConfig.value.frame || {});

const picList = computed<any[]>(() => {
    const list = moduleConfig.value.picList;
    return Array.isArray(list) ? list : [];
});

// 每屏张数：Admin 下发可能为字符串，模板只覆盖 1~3 列
const slidesPerView = computed(() => {
    const num = Number(moduleConfig.value.number);
    if (!Number.isFinite(num) || num < 1) return 1;
    return Math.min(Math.floor(num), 3);
});

const slideGap = computed(() => {
    const gap = Number(moduleConfig.value.slideGap);
    if (!Number.isFinite(gap) || gap < 0) return 12;
    return gap;
});

const moduleClasses = computed(() => [`dec-banner2--num-${slidesPerView.value}`]);

const cssVars = computed(() => {
    const f = frame.value;
    return {
        "--dec-box-padding": `${Number(f.boxPadding) || 0}px`,
        "--dec-box-padding-top": `${Number(f.boxPaddingTop) || 0}px`,
        "--dec-box-padding-bottom": `${Number(f.boxPaddingBottom) || 0}px`,
        "--dec-box-radius": `${Number(f.boxRadius) || 0}px`,
        "--dec-bg-color": f.backgroundColor || "transparent",
        "--dec-inner-padding": `${Number(f.leftAndRightPadding) || 0}px`,
        "--dec-slide-gap": `${slideGap.value}px`
    };
});

const swiperOption = computed(() => {
    const num = slidesPerView.value;
    return {
        autoplay: {
            delay: 5000,
            disableOnInteraction: false,
            pauseOnMouseEnter: true
        },
        pagination: {
            clickable: true,
            enabled: true
        },
        navigation: true,
        effect: "slide",
        loop: true,
        slidesPerView: num,
        slidesPerGroup: num,
        spaceBetween: num > 1 ? slideGap.value : 0
    };
});

const swiperKey = computed(() => `${picList.value.length}-${slidesPerView.value}-${slideGap.value}`);

function formatImage(url?: string): string {
    if (!url) return "";
    return imageFormat(url);
}

function getAltText(item: any): string {
    const raw =
        item?.picTitle?.toString().trim() ||
        item?.picDesc?.toString().trim() ||
        item?.picName?.toString().trim() ||
        siteName.value;
    return truncateSeoText(raw, 125);
}
</script>

<style scoped lang="less">
.dec-banner2 {
    box-sizing: border-box;
    width: 100%;
    padding-left: var(--dec-box-padding, 0);
    padding-right: var(--dec-box-padding, 0);
    padding-top: var(--dec-box-padding-top, 0);
    padding-bottom: var(--dec-box-padding-bottom, 0);

    &__inner {
        position: relative;
        box-sizing: border-box;
        width: 100%;
        overflow: hidden;
        border-radius: var(--dec-box-radius, 8px);
        background-color: var(--dec-bg-color, transparent);
        padding-left: var(--dec-inner-padding, 0);
        padding-right: var(--dec-inner-padding, 0);

        :deep(.swiper) {
            width: 100%;
        }
    }

    &__item {
        width: 100%;

        &:hover {
            filter: brightness(105%);
        }
    }

    &__link {
        display: block;
        width: 100%;
        position: relative;
        overflow: hidden;
        border-radius: var(--dec-box-radius, 8px);
    }

    &__image {
        display: block;
        width: 100%;
        height: auto;
        object-fit: contain;
        object-position: center;
        background-color: #eeeeee;
        transition: opacity 0.2s;
    }
}

.dec-banner2__inner :deep(.swiper-pagination-bullet) {
    width: 6px;
    height: 6px;
    background: #ffffffcc;
    opacity: 0.2;
}

.dec-banner2__inner :deep(.swiper-pagination-bullet-active) {
    background: #fff !important;
    opacity: 1 !important;
}

.dec-banner2__inner :deep(.swiper-button-prev),
.dec-banner2__inner :deep(.swiper-button-next) {
    box-sizing: border-box;
    width: 40px;
    height: 48px;
    line-height: 35px;
    z-index: 2;
    outline: none;
    background-color: rgba(0, 0, 0, 0.122);
    transition: background-color 0.2s ease;

    &:hover {
        background-color: rgba(0, 0, 0, 0.239);
    }
}

.dec-banner2__inner :deep(.swiper-button-prev) {
    left: 0;
    padding-right: 7px;
    border-radius: 0 24px 24px 0 !important;
}

.dec-banner2__inner :deep(.swiper-button-next) {
    right: 0;
    padding-left: 7px;
    border-radius: 24px 0 0 24px !important;
}

.dec-banner2__inner :deep(.swiper-button-prev:after),
.dec-banner2__inner :deep(.swiper-button-next:after) {
    font-family: "iconfont-pc";
    content: "\e70d";
    color: #fff;
    font-size: 14px;
}

.dec-banner2__inner :deep(.swiper-button-prev:after) {
    transform: rotate(180deg);
}

.dec-banner2__inner :deep(.swiper-button-prev.swiper-button-disabled),
.dec-banner2__inner :deep(.swiper-button-next.swiper-button-disabled) {
    pointer-events: fill;
}
</style>
