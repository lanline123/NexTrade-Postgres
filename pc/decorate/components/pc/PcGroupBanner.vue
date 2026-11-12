<template>
    <div v-if="picList.length" class="dec-group-banner" :style="cssVars">
        <div class="dec-group-banner__inner">
            <Swiper :key="swiperKey" :swiper-option="swiperOption" :item-list="slideList">
                <template #default="{ item }">
                    <div class="dec-group-banner__item">
                        <NuxtLink
                            class="dec-group-banner__link"
                            target="_blank"
                            :to="urlFormat(item.picLink)"
                            :title="getAltText(item)"
                            :aria-label="getAltText(item)"
                        >
                            <img class="dec-group-banner__image" loading="lazy" :src="formatImage(item.picUrl)" :alt="getAltText(item)" />
                        </NuxtLink>
                    </div>
                </template>
            </Swiper>
        </div>
    </div>
</template>

<script setup lang="ts">
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

// Admin 下发的 number 为字符串，且模板只覆盖 1~3 列
const slidesPerView = computed(() => {
    const num = Number(moduleConfig.value.number);
    if (!Number.isFinite(num) || num < 1) return 3;
    return Math.min(Math.floor(num), 3);
});

// 图片数少于 slidesPerView*2 时 Swiper 会自动关掉 loop，这里复制补齐以保持循环滚动
const slideList = computed<any[]>(() => {
    const list = picList.value;
    const minRequired = slidesPerView.value * 2;
    if (list.length === 0 || list.length >= minRequired) return list;

    const extended = [...list];
    while (extended.length < minRequired) {
        extended.push({ ...list[(extended.length - list.length) % list.length] });
    }
    return extended;
});

const cssVars = computed(() => {
    const f = frame.value;
    const height = Number(f.slideHeight) > 0 ? Number(f.slideHeight) : 195;

    return {
        "--dec-box-padding": `${Number(f.boxPadding) || 0}px`,
        "--dec-box-padding-top": `${Number(f.boxPaddingTop) || 0}px`,
        "--dec-box-padding-bottom": `${Number(f.boxPaddingBottom) || 0}px`,
        "--dec-box-radius": `${Number(f.boxRadius) || 0}px`,
        "--dec-bg-color": f.backgroundColor || "transparent",
        "--dec-slide-height": `${height}px`
    };
});

const swiperOption = computed(() => ({
    autoplay: {
        delay: 5000,
        disableOnInteraction: false,
        pauseOnMouseEnter: true
    },
    spaceBetween: 30,
    navigation: true,
    effect: "slide",
    loop: true,
    loopAdditionalSlides: slidesPerView.value,
    slidesPerView: slidesPerView.value,
    slidesPerGroup: slidesPerView.value
}));

const swiperKey = computed(() => `${slideList.value.length}-${slidesPerView.value}`);

function formatImage(url?: string): string {
    if (!url) return "";
    return imageFormat(url);
}

function getAltText(item: any): string {
    const raw =
        item?.picTitle?.toString().trim() ||
        item?.picName?.toString().trim() ||
        siteName.value;
    return truncateSeoText(raw, 125);
}
</script>

<style scoped lang="less">
.dec-group-banner {
    box-sizing: border-box;
    width: 100%;
    padding-left: var(--dec-box-padding, 0);
    padding-right: var(--dec-box-padding, 0);
    padding-top: var(--dec-box-padding-top, 0);
    padding-bottom: var(--dec-box-padding-bottom, 0);
    background-color: var(--dec-bg-color, transparent);

    &__inner {
        position: relative;
        width: 100%;
        height: var(--dec-slide-height, 195px);
        border-radius: var(--dec-box-radius, 0);
        overflow: hidden;

        :deep(.swiper) {
            width: 100%;
            height: 100%;
            user-select: none;
        }
    }

    &__item {
        display: flex;
        justify-content: center;
        height: var(--dec-slide-height, 195px);
        user-select: none;

        &:hover {
            filter: brightness(105%);
        }
    }

    &__link {
        display: block;
        width: 100%;
        height: 100%;
        position: relative;
        border-radius: var(--dec-box-radius, 0);
        overflow: hidden;
    }

    &__image {
        display: block;
        width: 100%;
        height: 100%;
        object-fit: cover;
        background-color: #eeeeee;
        transition: opacity 0.2s;
        pointer-events: none;
    }
}

.dec-group-banner__inner :deep(.swiper-button-prev),
.dec-group-banner__inner :deep(.swiper-button-next) {
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

.dec-group-banner__inner :deep(.swiper-button-prev) {
    left: 0;
    padding-right: 7px;
    border-radius: 0 24px 24px 0 !important;
}

.dec-group-banner__inner :deep(.swiper-button-next) {
    right: 0;
    padding-left: 7px;
    border-radius: 24px 0 0 24px !important;
}

.dec-group-banner__inner :deep(.swiper-button-prev:after),
.dec-group-banner__inner :deep(.swiper-button-next:after) {
    font-family: "iconfont-pc";
    content: "\e70d";
    color: #fff;
    font-size: 14px;
}

.dec-group-banner__inner :deep(.swiper-button-prev:after) {
    transform: rotate(180deg);
}

.dec-group-banner__inner :deep(.swiper-button-prev.swiper-button-disabled),
.dec-group-banner__inner :deep(.swiper-button-next.swiper-button-disabled) {
    pointer-events: fill;
}
</style>
