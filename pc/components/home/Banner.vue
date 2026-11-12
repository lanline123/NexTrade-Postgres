<template>
    <div>
        <!-- <template v-if="pageType == 1"> -->
        <template v-if="modelValue.bannerStyle == 1">
            <div class="banner container">
                <div class="inner">
                    <div class="acea-row row-between-wrapper">
                        <div class="left">
                            <CommonAllCate></CommonAllCate>
                        </div>
                        <div class="middle">
                            <div class="acea-row">
                                <div class="sliderBannerWrapper">
                                    <div class="slider sliderBanner">
                                        <div class="slider_list">
                                            <div class="slider_wrapper">
                                                <swiper v-model:swiperOption="swiperOption" :itemList="modelValue.picList">
                                                    <template v-slot:default="{ item, index }">
                                                        <li class="slider_item">
                                                            <NuxtLink
                                                                :to="urlFormat(item.picLink)"
                                                                class="item__ad_lk"
                                                                target="_blank"
                                                                :title="getAltText(item)"
                                                                :aria-label="getAltText(item)"
                                                            >
                                                                <el-image loading="lazy" :src="imageFormat(item.picUrl)" class="item-img" :alt="getAltText(item)" />
                                                            </NuxtLink>
                                                        </li>
                                                    </template>
                                                </swiper>
                                            </div>
                                        </div>
                                    </div>
                                </div>
                                <div class="sliderRecommendWrapper">
                                    <div class="slider sliderRecommend">
                                        <div class="slider_list">
                                            <div class="slider_wrapper">
                                                <swiper :itemList="list2" :swiperOption="swiperRecompic">
                                                    <template v-slot:default="{ item }">
                                                        <li class="slider_item">
                                                            <div class="item-recommend">
                                                                <NuxtLink
                                                                    v-for="(items, index) in item"
                                                                    :key="index"
                                                                    class="recommend-item"
                                                                    :to="urlFormat(items.picLink)"
                                                                    target="_blank"
                                                                    :title="getAltText(items)"
                                                                    :aria-label="getAltText(items)"
                                                                >
                                                                    <el-image loading="lazy" :src="imageFormat(items.picUrl)" class="recommend-item__image" :alt="getAltText(items)" />
                                                                </NuxtLink>
                                                            </div>
                                                        </li>
                                                    </template>
                                                </swiper>
                                            </div>
                                        </div>
                                    </div>
                                </div>
                            </div>
                        </div>
                        <div class="right">
                            <CommonSmallPersonalCenter :value="modelValue"></CommonSmallPersonalCenter>
                        </div>
                    </div>
                </div>
            </div>
        </template>
        <template v-else>
            <div class="banner" :class="{ wide: modelValue.bannerStyle == 2 }">
                <div class="inner container flex justify-start">
                    <div class="acea-row row-between-wrapper" v-if="modelValue.showCat == 1">
                        <div class="left">
                            <CommonAllCate></CommonAllCate>
                        </div>
                    </div>
                </div>
                <HomeBannerSimple :value="modelValue"></HomeBannerSimple>
            </div>
        </template>
    </div>
</template>
<script lang="ts" setup>
import { ref } from "vue";
import { imageFormat, urlFormat } from "@/utils/format";
import { useCommonStore } from "~/store/common";
const commonStore: any = useCommonStore();

commonStore.loadNav();

const pageType = computed(() => {
    return commonStore.decoratePageConfig?.headerStyle;
});

const siteName = computed(() => commonStore.shopTitle || commonStore.shopName || "");

const getAltText = (item: any) => {
    return (
        item?.picTitle?.toString().trim() ||
        item?.picDesc?.toString().trim() ||
        item?.picName?.toString().trim() ||
        siteName.value
    );
};

const props = defineProps({
    modelValue: {
        type: Object,
        default: {}
    }
});

const list2 = computed(() => {
    return groupArray(props.modelValue.picList2);
});

function groupArray(arr: any) {
    let result = [];
    for (let i = 0; i < arr.length; i += 3) {
        let group = arr.slice(i, i + 3);
        result.push(group);
    }
    return result;
}

const swiperOption = ref<any>({
    // Swiper的配置选项
    // 根据实际需求进行配置
    // 例如：autoplay, loop, navigation等
    autoplay: {
        delay: 5000,
        disableOnInteraction: false, //用户操作swiper之后，是否禁止autoplay
        pauseOnMouseEnter: true //鼠标置于swiper是否时暂停自动切换
    },
    navigation: true,
    effect: "slide",
    loop: true
});

const swiperRecompic = ref<any>({
    autoplay: {
        delay: 5000,
        disableOnInteraction: false, //用户操作swiper之后，是否禁止autoplay
        pauseOnMouseEnter: true //鼠标置于swiper是否时暂停自动切换
    },
    pagination: false,
    navigation: true,
    effect: "fade",
    loop: true
});
</script>
<style lang="less" scoped>
.acea-row {
    display: flex;
    flex-wrap: wrap;
}

.acea-row.row-middle {
    align-items: center;
}

.acea-row.row-right {
    justify-content: flex-end;
}

.acea-row.row-center-wrapper {
    align-items: center;
    justify-content: center;
}

.acea-row.row-between-wrapper {
    align-items: center;
    justify-content: space-between;
}

.banner {
    position: relative;
    margin-top: 10px;
    &.wide {
        margin-top: 0;
        .inner {
            height: 550px;
            .left {
                box-shadow: rgba(0, 0, 0, 0.1) 0px 0px 36px;
            }
        }
    }
}

.inner {
    position: relative;
    height: 470px;
}

.inner .left {
    width: 190px;
    height: 480px;
    margin-top: -10px;
    margin-right: 10px;
    position: relative;
    background-color: #fff;
    z-index: 2;
}

// banner
.inner .middle {
    position: relative;
    width: 790px;
    height: 470px;

    margin-right: 10px;
    overflow: hidden;
    animation: skeletonShow 0.3s ease both;
}

.inner .middle .slider {
    position: relative;
}

.inner .middle .sliderBannerWrapper {
    width: 590px;
    margin-right: 10px;
    overflow: hidden;
}

.inner .middle :deep(.swiper-button-prev),
.inner .middle :deep(.swiper-button-next) {
    width: 25px;
    height: 35px;
    line-height: 35px;
    background-color: #d9d9d9;
    background-color: rgba(0, 0, 0, 0.15);
    z-index: 2;
    border: none;
    outline: none;
    transition: background-color 0.2s ease;
}

.inner .middle :deep(.swiper-button-prev) {
    left: 0;
    border-top-right-radius: 2px;
    border-bottom-right-radius: 2px;
}

.inner .middle :deep(.swiper-button-next) {
    right: 0;
    border-top-left-radius: 2px;
    border-bottom-left-radius: 2px;
}

.inner .middle :deep(.swiper-button-prev:after),
.inner .middle :deep(.swiper-button-next:after) {
    color: #fff;
    font-size: 14px;
    font-weight: 700;
}

.inner .middle :deep(.swiper-button-prev.swiper-button-disabled),
.inner .middle :deep(.swiper-button-next.swiper-button-disabled) {
    pointer-events: fill;
}

.inner .middle .slider_item {
    height: 100%;
    min-height: 1px;
}

.inner .middle .slider_item .item__ad_lk {
    display: inline-block;
    margin-right: 10px;
    width: 590px;
    height: 470px;
    overflow: hidden;
    position: relative;
}

.inner .middle .slider_item .item__ad_lk .item-img {
    display: block;
    width: 100%;
    height: 100%;
    transition: opacity 0.2s;
    background-color: #eeeeee;
}

.inner .middle .sliderBanner :deep(.swiper-pagination) {
    padding-left: 25px;
    padding-bottom: 10px;
    text-align: left;
}

.inner .middle .sliderBanner :deep(.swiper-pagination-bullet) {
    position: relative;
    display: inline-block;
    transition: background 0.2s ease;
    width: 8px;
    height: 8px;
    margin-right: 4px;
    border: 1px solid rgba(0, 0, 0, 0.05);
    background: rgba(255, 255, 255, 0.8);
}

.inner .middle .sliderBanner :deep(.swiper-pagination-bullet-active) {
    width: 9px;
    height: 9px;
    top: 2px;
    left: 0;
    border: 3px solid rgba(0, 0, 0, 0.1);
}

.inner .middle .sliderBanner :deep(.swiper-pagination-bullet-active:before) {
    content: " ";
    width: 9px;
    height: 9px;
    position: absolute;
    border-radius: 50%;
    left: 0;
    background-color: white;
}

.inner .middle .sliderRecommendWrapper {
    width: 190px;
    overflow: hidden;
}

.inner .middle .sliderRecommendWrapper .item-recommend {
    width: 190px;
    vertical-align: top;
    display: inline-block;
    height: 100%;
}

.inner .middle .sliderRecommendWrapper .item-recommend .recommend-item {
    height: 150px;
    display: block;
    margin-bottom: 10px;
}

.inner .middle .sliderRecommendWrapper .item-recommend .recommend-item:last-child {
    margin-bottom: 0;
}

.inner .middle .sliderRecommendWrapper .item-recommend .recommend-item__image {
    position: relative;
    width: 100%;
    height: 100%;
    background-color: #eeeeee;
}

.inner .right {
    position: relative;
    width: 190px;
    height: 470px;
}
</style>
