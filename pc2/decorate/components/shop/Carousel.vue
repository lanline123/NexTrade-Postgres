<template>
    <div class="carousel-list" :class="['carousel-list-bt-' + module.picPageType]">
        <div class="carousel-viewport" :style="{ borderRadius: Number(module.picRadioStyle) === 2 ? '8px' : '0px' }">
            <swiper class="slider_wrapper" v-model:swiperOption="swiperOption" :itemList="adjustedPicList">
                <template #default="{ item }">
                    <div class="image-ad-item">
                        <template v-for="img in item" :key="img.picUrl">
                            <NuxtLink
                                target="_blank"
                                :to="urlFormat(img.picLink)"
                                class="image-container"
                                :style="{ padding: module.imgPadding + 'px', width: 100 / module.swiperPreView + '%' }"
                            >
                                <img :class="[Number(module.picRadioStyle) === 2 ? 'image-box-radius' : '']" class="image-box" :src="imageFormat(img.picUrl)" alt="" />
                            </NuxtLink>
                        </template>
                    </div>
                </template>
            </swiper>
        </div>
    </div>
</template>
<script setup lang="ts">
import { ref, computed } from "vue";
import { imageFormat, urlFormat } from "~/utils/format";
import Swiper from "~/decorate/components/swiper/Swiper.vue";

const props = defineProps({
    module: { type: Object, default: () => ({}) }
});

const swiperOption = ref<Record<string, unknown>>({
    autoplay: {
        delay: 30000,
        disableOnInteraction: false,
        pauseOnMouseEnter: true
    },
    pagination: {
        clickable: true
    },
    loop: true
});

const placeholderItem: Record<string, unknown> = { __isPlaceholder: true };

const adjustedPicList = computed(() => {
    const pageSize = props.module.swiperPreView;
    const totalItems = props.module.picList?.length;
    const pageCount = Math.ceil(props.module.picList?.length / pageSize);
    const adjustedLists: unknown[][] = [];

    for (let i = 0; i < pageCount; i++) {
        const page: unknown[] = [];
        for (let j = 0; j < pageSize; j++) {
            const index = i * pageSize + j;
            if (index < totalItems) {
                page.push(props.module.picList[index]);
            } else {
                page.push(placeholderItem);
            }
        }
        adjustedLists.push(page);
    }

    return adjustedLists;
});

const swiperPageColor = computed(() => {
    return props.module.swiperPageColor || "#333";
});
</script>
<style scoped lang="less">
.carousel-list {
    display: flex;
    width: 100%;
    flex: 1;

    .carousel-viewport {
        width: 100%;
        overflow: hidden;
    }

    .slider_wrapper {
        width: 100%;
        .image-ad-item {
            width: 100%;
            display: flex;
            flex-direction: row;

            .image-container {
                box-sizing: border-box;

                .image-box {
                    width: 100%;
                    height: auto;
                    display: block;
                    object-fit: cover;
                }
            }
        }
    }
}

.carousel-list :deep(.swiper-button-prev),
.carousel-list :deep(.swiper-button-next) {
    z-index: 2;
    border: none;
    outline: none;
    transition: background-color 0.2s ease;
}

.carousel-list :deep(.swiper-button-prev) {
    left: 10px;
    border-top-right-radius: 2px;
    border-bottom-right-radius: 2px;
}

.carousel-list :deep(.swiper-button-next) {
    right: 10px;
    border-top-left-radius: 2px;
    border-bottom-left-radius: 2px;
}

.carousel-list :deep(.swiper-button-prev:after),
.carousel-list :deep(.swiper-button-next:after) {
    color: #e5e5e5;
    font-size: 24px;
    font-weight: bolder;
}

.carousel-list :deep(.swiper-pagination) {
    padding-left: 25px;
    width: 100%;
    text-align: center;
    position: absolute;
    bottom: 25px;
    z-index: 10;
}

.carousel-list :deep(.swiper-pagination-bullet) {
    position: relative;
    display: inline-block;
    transition: background 0.2s ease;
    background-color: #fff;
}

.carousel-list-bt-1 :deep(.swiper-pagination-bullet) {
    width: 12px;
    height: 6px;
    border-radius: 2px;
    margin: 0 4px;
}

.carousel-list-bt-1 :deep(.swiper-pagination-bullet-active) {
    width: 12px;
    height: 6px;
    border-radius: 2px;
    margin: 0 4px;
}

.carousel-list-bt-2 :deep(.swiper-pagination-bullet) {
    width: 12px;
    height: 12px;
    border-radius: 50%;
    margin: 0 3px;
}

.carousel-list-bt-2 :deep(.swiper-pagination-bullet-active) {
    width: 12px;
    height: 12px;
    border-radius: 50%;
    margin: 0 3px;
}

.carousel-list-bt-3 :deep(.swiper-pagination) {
    text-align: right;
    background-color: rgba(0, 0, 0, 0.3);
    border-radius: 100px;
    padding: 3px 5px;
    display: flex;
    width: auto;
    font-size: 12px;
    position: absolute;
    bottom: 15px;
    left: 8px;
    height: 15px;
    align-items: center;
}

.carousel-list-bt-3 :deep(.swiper-pagination-bullet) {
    width: 12px;
    height: 12px;
    opacity: 1;
    border-radius: 50%;
    margin: 0 3px;
}

.carousel-list-bt-3 :deep(.swiper-pagination-bullet-active) {
    width: 12px;
    height: 12px;
    opacity: 1;
    border-radius: 50%;
    margin: 0 3px;
}

:deep(.swiper-pagination-bullet-active) {
    background: v-bind("swiperPageColor") !important;
}
</style>
