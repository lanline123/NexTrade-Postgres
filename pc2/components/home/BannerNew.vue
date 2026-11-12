<template>
    <div>
        <div class="banner container">
            <div class="inner">
                <swiper v-model:swiperOption="swiperOption" :itemList="modelValue.picList">
                    <template v-slot:default="{ item, index }">
                        <li class="slider_item" v-if="slideNum == 1">
                            <NuxtLink
                                :to="urlFormat(item.picLink)"
                                class="item__ad_lk"
                                target="_blank"
                                :title="getAltText(item)"
                                :aria-label="getAltText(item)"
                            >
                                <el-image loading="lazy" fit="fill" :src="productImage(item)" class="item-img" :alt="getAltText(item)" />
                            </NuxtLink>
                        </li>
                        <li class="slider_item" v-if="slideNum == 2">
                            <NuxtLink
                                :to="urlFormat(item.picLink)"
                                class="item__ad_lk-2"
                                target="_blank"
                                :title="getAltText(item)"
                                :aria-label="getAltText(item)"
                            >
                                <el-image loading="lazy" fit="fill" :src="productImage(item)" class="item-img-2" :alt="getAltText(item)" />
                            </NuxtLink>
                        </li>
                        <li class="slider_item" v-if="slideNum == 3">
                            <NuxtLink
                                :to="urlFormat(item.picLink)"
                                class="item__ad_lk-3"
                                target="_blank"
                                :title="getAltText(item)"
                                :aria-label="getAltText(item)"
                            >
                                <el-image loading="lazy" fit="fill" :src="productImage(item)" class="item-img-3" :alt="getAltText(item)" />
                            </NuxtLink>
                        </li>
                    </template>
                </swiper>
            </div>
        </div>
    </div>
</template>
<script lang="ts" setup>
import { ref } from "vue";
import { productImage, urlFormat } from "@/utils/format";
import { truncateSeoText } from "@/utils/seo";
import { useCommonStore } from "~/store/common";

const props = defineProps({
    modelValue: {
        type: Object,
        default: {}
    },
    slideNum: {
        type: Number,
        default: 1
    }
});

const commonStore: any = useCommonStore();
const siteName = computed(() => commonStore.shopTitle || commonStore.shopName || "");

const getAltText = (item: any) => {
    const raw =
        item?.picTitle?.toString().trim() ||
        item?.picDesc?.toString().trim() ||
        item?.picName?.toString().trim() ||
        siteName.value;
    return truncateSeoText(raw, 125);
};

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
    pagination: {
        clickable: true,
        enabled: true
    },
    slidesPerGroup: Number(props.slideNum),
    slidesPerView: Number(props.slideNum),
    navigation: true,
    effect: "slide",
    loop: true
});

watch(
    () => props.slideNum,
    (newVal) => {
        if (newVal < 3) {
            swiperOption.value.slidesPerGroup = Number(newVal);
            swiperOption.value.slidesPerView = Number(newVal);
        }
    }
);
</script>
<style lang="less" scoped>
.banner {
    position: relative;
    margin-top: 32px;
    width: 100%;
    height: 280px;
    .inner {
        position: relative;
        width: 100%;
        height: 280px;
        border-radius: 8px;
        :deep(.swiper) {
            width: 100%;
            height: 280px;
            margin-right: 10px;
            border-radius: 8px;
            overflow: hidden;
        }
        .slider_item {
            display: flex;
            justify-content: center;
            height: 280px;
            min-height: 1px;
            &.start {
                justify-content: start;
            }
            &.end {
                justify-content: end;
            }
            &:hover {
                filter: brightness(105%);
            }
            .item__ad_lk {
                display: inline-block;
                width: 100%;
                height: 100%;
                overflow: hidden;
                position: relative;
                .item-img {
                    display: block;
                    width: 100%;
                    height: 100%;
                    transition: opacity 0.2s;
                    background-color: #eeeeee;
                }
            }
            .item__ad_lk-2 {
                display: inline-block;
                width: 96%;
                height: 100%;
                position: relative;
                border-radius: 8px;
                overflow: hidden;
                .item-img-2 {
                    display: block;
                    width: 100%;
                    height: 100%;
                    transition: opacity 0.2s;
                    background-color: #eeeeee;
                }
            }
            .item__ad_lk-3 {
                display: inline-block;
                width: 96%;
                height: 100%;
                position: relative;
                border-radius: 8px;
                overflow: hidden;
                .item-img-3 {
                    display: block;
                    width: 100%;
                    height: 100%;
                    transition: opacity 0.2s;
                    background-color: #eeeeee;
                }
            }
        }
    }
}

:deep(.swiper-pagination-bullet) {
    width: 6px;
    height: 6px;
    background: #ffffffcc;
    opacity: 0.2;
}
:deep(.swiper-pagination-bullet-active) {
    background: #fff !important;
    opacity: 1 !important;
}

.inner :deep(.swiper-button-prev) {
    box-sizing: border-box;
    left: 0;
    background-color: rgba(0, 0, 0, 0.122);
    width: 40px;
    height: 48px;
    padding-right: 7px;
    line-height: 35px;
    border-radius: 0 24px 24px 0 !important;
    z-index: 2;
    outline: none;
    transition: background-color 0.2s ease;
    &:hover {
        background-color: rgba(0, 0, 0, 0.239);
    }
}

.inner :deep(.swiper-button-next) {
    box-sizing: border-box;
    right: 0;
    background-color: rgba(0, 0, 0, 0.122);
    width: 40px;
    height: 48px;
    padding-left: 7px;
    line-height: 35px;
    z-index: 2;
    border-radius: 24px 0 0 24px !important;
    outline: none;
    transition: background-color 0.2s ease;
    &:hover {
        background-color: rgba(0, 0, 0, 0.239);
    }
}

.inner :deep(.swiper-button-prev:after),
.inner :deep(.swiper-button-next:after) {
    font-family: "iconfont-pc";
    content: "\e70d";
    color: #fff;
    font-size: 14px;
}

.inner :deep(.swiper-button-prev:after) {
    transform: rotate(180deg);
}

.inner :deep(.swiper-button-prev.swiper-button-disabled),
.inner :deep(.swiper-button-next.swiper-button-disabled) {
    pointer-events: fill;
}
</style>
