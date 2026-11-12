<template>
    <div class="dec-category-nav3" :style="cssVars">
        <div class="dec-category-nav3__top">
            <div class="dec-category-nav3__title">{{ titleText }}</div>
            <NuxtLink v-if="showMore" class="dec-category-nav3__more" :to="urlFormat(moduleConfig.moreLink)">
                {{ moduleConfig.showMoreText || $t("更多") }}
                <SvgIcon class="dec-category-nav3__more-icon" name="arrow-right" width="16px" height="16px" />
            </NuxtLink>
        </div>

        <div v-if="picList.length" class="dec-category-nav3__list">
            <Swiper :swiper-option="swiperOption" :item-list="picList" style="width: 100%">
                <template #default="{ item }">
                    <div class="dec-category-nav3__item">
                        <NuxtLink :to="urlFormat(item.picLink)" :title="getAltText(item)" target="_blank">
                            <div class="dec-category-nav3__image" :style="{ backgroundColor: getImageBg(item) }">
                                <img class="dec-category-nav3__img" loading="lazy" :src="formatImage(item.picUrl || item.picThumb)" :alt="getAltText(item)" />
                            </div>
                            <div class="dec-category-nav3__name">{{ item.picTitle }}</div>
                            <div v-if="item.picDesc" class="dec-category-nav3__desc">
                                <SvgIcon name="hot" width="16px" height="16px" />
                                <div class="dec-category-nav3__desc-text">{{ item.picDesc }}</div>
                            </div>
                        </NuxtLink>
                    </div>
                </template>
            </Swiper>
        </div>
    </div>
</template>

<script setup lang="ts">
/**
 * PC 分类导航3（新套 kebab 模块 pc-category-nav3）
 * 由旧套 pcCategoryNav3 迁移而来：图片列表与链接结构不变，容器样式改由 frame 驱动。
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

const titleText = computed(() => moduleConfig.value.titleText || "");
const showMore = computed(() => Number(moduleConfig.value.showMore) === 1);

const picList = computed<any[]>(() => {
    const list = moduleConfig.value.picList;
    return Array.isArray(list) ? list : [];
});

const cssVars = computed(() => {
    const f = frame.value;
    const bgColor = moduleConfig.value.backgroundColor || f.backgroundColor || "rgba(0, 0, 0, 0.02)";
    return {
        "--dec-box-padding": `${Number(f.boxPadding) || 0}px`,
        "--dec-box-padding-top": `${Number(f.boxPaddingTop) || 0}px`,
        "--dec-box-padding-bottom": `${Number(f.boxPaddingBottom) || 0}px`,
        "--dec-box-radius": `${Number(f.boxRadius) || 0}px`,
        "--dec-bg-color": bgColor,
        "--dec-box-height": `${Number(f.boxHeight) > 0 ? Number(f.boxHeight) : 352}px`
    };
});

const swiperOption = computed(() => ({
    autoplay: {
        delay: 5000,
        disableOnInteraction: false,
        pauseOnMouseEnter: true
    },
    slidesPerGroup: 6,
    slidesPerView: 6,
    pagination: false,
    navigation: true,
    effect: "slide",
    loop: true
}));

function getImageBg(item: any): string {
    return item?.imageBackgroundColor || moduleConfig.value.imageBackgroundColor || "var(--primary-light-9)";
}

function formatImage(url?: string): string {
    if (!url) return "";
    return imageFormat(url);
}

function getAltText(item: any): string {
    const raw = item?.picTitle?.toString().trim() || item?.picName?.toString().trim() || siteName.value;
    return truncateSeoText(raw, 125);
}
</script>

<style scoped lang="less">
.dec-category-nav3 {
    display: flex;
    flex-direction: column;
    box-sizing: border-box;
    width: 100%;
    height: var(--dec-box-height, 352px);
    padding-left: var(--dec-box-padding, 0);
    padding-right: var(--dec-box-padding, 0);
    padding-top: var(--dec-box-padding-top, 0);
    padding-bottom: var(--dec-box-padding-bottom, 0);
    border-radius: var(--dec-box-radius, 8px);
    background-color: var(--dec-bg-color, rgba(0, 0, 0, 0.02));
    overflow: hidden;

    &__top {
        display: flex;
        justify-content: space-between;
        align-items: center;
        padding: 20px 24px;
    }

    &__title {
        font-size: 24px;
        font-weight: 700;
        color: #1a1a1a;
    }

    &__more {
        display: flex;
        align-items: center;
        font-size: 18px;
        color: #1a1a1a;
    }

    &__more-icon {
        margin-left: 8px;
    }

    &__list {
        position: relative;
        z-index: 1;
        flex: 1;
        display: flex;
        width: 100%;
        overflow: hidden;
    }

    &__item {
        box-sizing: border-box;
        position: relative;
        z-index: 1;
        width: 100%;
        height: 100%;
        // 14px 对称沟槽：slidesPerView:6 下列宽随容器伸缩，图片铺满列宽，首尾与容器边距一致
        padding: 0 14px;

        a {
            display: flex;
            flex-direction: column;
            height: 100%;
        }

        &:hover {
            .dec-category-nav3__img {
                filter: brightness(105%);
            }

            .dec-category-nav3__name {
                color: var(--main-bg);
            }
        }
    }

    &__image {
        width: 100%;
        aspect-ratio: 1;
        // 容器高度由 --dec-box-height 配置：正方形放得下就保持正方形，
        // 放不下时允许压扁而不是把描述行挤出 overflow
        flex: 0 1 auto;
        min-height: 0;
        background: var(--primary-light-9);
        border-radius: 4px;
        overflow: hidden;
    }

    &__img {
        display: block;
        width: 100%;
        height: 100%;
        object-fit: cover;
        transition: filter 0.2s linear;
    }

    &__name {
        flex: none;
        width: 100%;
        height: 20px;
        margin-top: 12px;
        margin-bottom: 8px;
        text-align: center;
        overflow: hidden;
        white-space: nowrap;
        text-overflow: ellipsis;
        color: #1a1a1a;
        font-size: 16px;
        font-weight: 400;
        line-height: 20px;
        text-transform: capitalize;
    }

    &__desc {
        display: flex;
        flex: none;
        justify-content: center;
        align-items: center;
        width: 100%;
    }

    &__desc-text {
        margin-left: 4px;
        color: #888b94;
        font-size: 16px;
        line-height: 20px;
        overflow: hidden;
        white-space: nowrap;
        text-overflow: ellipsis;
    }
}

.dec-category-nav3__list :deep(.swiper) {
    box-sizing: border-box;
    width: 100%;
    padding: 0 12px;
}

.dec-category-nav3__list :deep(.swiper-slide) {
    width: 248px;
}

.dec-category-nav3__list :deep(.swiper-button-prev),
.dec-category-nav3__list :deep(.swiper-button-next) {
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

.dec-category-nav3__list :deep(.swiper-button-prev) {
    left: 0;
    padding-right: 7px;
    border-radius: 0 24px 24px 0 !important;
}

.dec-category-nav3__list :deep(.swiper-button-next) {
    right: 0;
    padding-left: 7px;
    border-radius: 24px 0 0 24px !important;
}

.dec-category-nav3__list :deep(.swiper-button-prev:after),
.dec-category-nav3__list :deep(.swiper-button-next:after) {
    font-family: "iconfont-pc";
    content: "\e70d";
    color: #fff;
    font-size: 14px;
}

.dec-category-nav3__list :deep(.swiper-button-prev:after) {
    transform: rotate(180deg);
}

.dec-category-nav3__list :deep(.swiper-button-prev.swiper-button-disabled),
.dec-category-nav3__list :deep(.swiper-button-next.swiper-button-disabled) {
    pointer-events: fill;
}
</style>
