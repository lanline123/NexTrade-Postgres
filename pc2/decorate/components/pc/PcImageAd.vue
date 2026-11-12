<template>
    <div v-if="picList.length" class="dec-image-ad" :class="moduleClasses" :style="cssVars">
        <div class="dec-image-ad__content">
            <Title v-if="moduleConfig.title?.showTitle == 1" :module="moduleConfig.title" />

            <template v-if="Number(moduleConfig.picType) === 2">
                <div class="dec-image-ad__swiper">
                    <Swiper :key="swiperKey" :swiper-option="swiperOption" :item-list="picList">
                        <template #default="{ item }">
                            <div class="dec-image-ad__item">
                                <NuxtLink class="dec-image-ad__link" target="_blank" :to="urlFormat(item.picLink)">
                                    <img class="dec-image-ad__image" :src="formatImage(item.picUrl)" alt="" />
                                </NuxtLink>
                            </div>
                        </template>
                    </Swiper>
                </div>
            </template>

            <template v-else>
                <div class="dec-image-ad__list">
                    <div v-for="(pic, index) in picList" :key="index" class="dec-image-ad__item">
                        <NuxtLink class="dec-image-ad__link" target="_blank" :to="urlFormat(pic.picLink)">
                            <img class="dec-image-ad__image" :src="formatImage(pic.picUrl)" alt="" />
                        </NuxtLink>
                    </div>
                </div>
            </template>
        </div>
    </div>
</template>

<script setup lang="ts">
import { computed } from "vue";
import Swiper from "~/decorate/components/swiper/Swiper.vue";
import Title from "~/decorate/components/shop/Title.vue";
import { imageFormat, urlFormat } from "~/utils/format";

const props = defineProps({
    module: { type: Object, default: () => ({}) }
});

const moduleConfig = computed(() => props.module || {});
const picList = computed(() => moduleConfig.value.picList || []);

const moduleClasses = computed(() => {
    const data = moduleConfig.value;
    const classes = [`dec-image-ad--layout-${data.picType ?? 1}`];

    if (Number(data.picType) === 2) {
        classes.push(`dec-image-ad--indicator-${data.picPageType ?? 1}`);
    }
    if (Number(data.picRadioStyle) === 2) {
        classes.push("dec-image-ad--rounded");
    }
    if (Number(data.isFluxWidth) === 1) {
        classes.push("dec-image-ad--flux");
    }
    return classes;
});

const cssVars = computed(() => {
    const data = moduleConfig.value;
    const frame = data.frame || {};
    const isFluxWidth = Number(data.isFluxWidth) === 1;

    return {
        "--dec-box-padding": `${isFluxWidth ? 0 : (frame.boxPadding ?? 10)}px`,
        "--dec-box-padding-top": `${frame.boxPaddingTop ?? 5}px`,
        "--dec-box-padding-bottom": `${frame.boxPaddingBottom ?? 5}px`,
        "--dec-box-radius": `${frame.boxRadius ?? 0}px`,
        "--dec-bg-color": frame.backgroundColor ?? "#ffffff",
        "--dec-inner-padding": `${data.imgPadding ?? 0}px`,
        "--dec-item-gap": `${data.imgPadding ?? 0}px`,
        "--dec-item-radius": Number(data.picRadioStyle) === 2 ? "8px" : "0",
        "--dec-indicator-color": data.swiperPageColor ?? "#333333"
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
    slidesPerView: Number(moduleConfig.value.swiperPreView) || 1
}));

const swiperKey = computed(() => `${picList.value.length}-${Number(moduleConfig.value.swiperPreView) || 1}`);

function formatImage(url?: string): string {
    if (!url) return "";
    return imageFormat(url);
}
</script>

<style scoped lang="less">
.dec-image-ad {
    box-sizing: border-box;
    padding-left: var(--dec-box-padding, 10px);
    padding-right: var(--dec-box-padding, 10px);
    padding-top: var(--dec-box-padding-top, 5px);
    padding-bottom: var(--dec-box-padding-bottom, 5px);
    background-color: var(--dec-bg-color, transparent);
    border-radius: var(--dec-box-radius, 0);

    &__content {
        background-color: var(--dec-bg-color, #ffffff);
        border-radius: var(--dec-box-radius, 0);
        overflow: hidden;
    }

    &__list {
        display: flex;
        flex-wrap: wrap;
        gap: var(--dec-item-gap, 0);
    }

    &__item {
        box-sizing: border-box;
        padding: var(--dec-inner-padding, 0);
        flex-shrink: 0;
    }

    &__swiper {
        border-radius: var(--dec-item-radius, 0);
        overflow: hidden;

        :deep(.swiper-pagination-bullet) {
            position: relative;
            display: inline-block;
            transition: background 0.2s ease;
            background-color: #ffffff;
        }

        :deep(.swiper-pagination-bullet-active) {
            background: var(--dec-indicator-color, #333333) !important;
        }
    }

    &__link {
        display: block;
        border-radius: var(--dec-item-radius, 0);
        overflow: hidden;
    }

    &__image {
        width: 100%;
        height: auto;
        display: block;
        object-fit: cover;
        border-radius: var(--dec-item-radius, 0);
    }
}

.dec-image-ad--layout-1 {
    .dec-image-ad__list {
        flex-direction: column;
    }

    .dec-image-ad__item {
        width: 100%;
    }
}

.dec-image-ad--layout-2 {
    .dec-image-ad__item {
        width: 100%;
    }
}

.dec-image-ad--layout-5 {
    .dec-image-ad__item {
        width: calc((100% - var(--dec-item-gap, 0px)) / 2);
    }
}

.dec-image-ad--layout-6 {
    .dec-image-ad__item {
        width: calc((100% - var(--dec-item-gap, 0px) * 2) / 3);
    }
}

.dec-image-ad--layout-7 {
    .dec-image-ad__item {
        width: calc((100% - var(--dec-item-gap, 0px) * 3) / 4);
    }
}

// 指示器样式一：短条（对齐 vortmall Carousel carousel-list-bt-1）
.dec-image-ad--indicator-1 {
    .dec-image-ad__swiper {
        :deep(.swiper-pagination) {
            width: 100%;
            text-align: center;
            position: absolute;
            left: 0;
            right: auto;
            bottom: 25px;
            z-index: 10;
            background: transparent;
        }
    }

    :deep(.swiper-pagination-bullet),
    :deep(.swiper-pagination-bullet-active) {
        width: 12px;
        height: 6px;
        border-radius: 2px;
        margin: 0 4px;
    }
}

// 指示器样式二：圆点（对齐 vortmall Carousel carousel-list-bt-2）
.dec-image-ad--indicator-2 {
    .dec-image-ad__swiper {
        :deep(.swiper-pagination) {
            width: 100%;
            text-align: center;
            position: absolute;
            left: 0;
            right: auto;
            bottom: 25px;
            z-index: 10;
            background: transparent;
        }
    }

    :deep(.swiper-pagination-bullet),
    :deep(.swiper-pagination-bullet-active) {
        width: 12px;
        height: 12px;
        border-radius: 50%;
        margin: 0 3px;
    }
}

// 指示器样式三：右下角半透明胶囊（勿用 width:100%，避免背景条撑满父级）
.dec-image-ad--indicator-3 {
    .dec-image-ad__swiper {
        :deep(.swiper-pagination) {
            left: auto !important;
            right: 8px;
            bottom: 15px;
            width: auto !important;
            max-width: none;
            transform: none !important;
            display: inline-flex;
            align-items: center;
            justify-content: center;
            padding: 3px 5px;
            height: 15px;
            border-radius: 100px;
            background-color: rgba(0, 0, 0, 0.3);
            z-index: 10;
        }
    }

    :deep(.swiper-pagination-bullet),
    :deep(.swiper-pagination-bullet-active) {
        width: 12px;
        height: 12px;
        margin: 0 3px;
        border-radius: 50%;
        opacity: 1;
    }
}
</style>
