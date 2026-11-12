<template>
    <div>
        <div class="banner container">
            <div class="inner">
                <swiper 
                    v-model:swiperOption="swiperOption" 
                    :itemList="extendedPicList"
                    :key="`swiper-${modelValue.number}-${(modelValue.picList || []).length}`"
                >
                    <template v-slot:default="{ item, index }">
                        <li class="slider_item" v-if="modelValue.number == 1">
                            <NuxtLink :to="urlFormat(item.picLink)" class="item__ad_lk" target="_blank">
                                <el-image loading="lazy" fit="fill" :src="productImage(item)" class="item-img" />
                            </NuxtLink>
                        </li>
                        <li class="slider_item" v-if="modelValue.number == 2">
                            <NuxtLink :to="urlFormat(item.picLink)" class="item__ad_lk-2" target="_blank">
                                <el-image loading="lazy" fit="fill" :src="productImage(item)" class="item-img-2" />
                            </NuxtLink>
                        </li>
                        <li class="slider_item" v-if="modelValue.number == 3">
                            <NuxtLink :to="urlFormat(item.picLink)" class="item__ad_lk-3" target="_blank">
                                <el-image loading="lazy" fit="fill" :src="productImage(item)" class="item-img-3" />
                            </NuxtLink>
                        </li>
                    </template>
                </swiper>
            </div>
        </div>
    </div>
</template>
<script lang="ts" setup>
import { ref, computed } from "vue";
import { productImage, urlFormat } from "@/utils/format";

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

// 扩展图片列表：当图片数量不足时，复制原有图片来补充
// 确保至少有 slidesPerView * 2 张图片，以支持循环模式正常工作
const extendedPicList = computed(() => {
    const originalList = props.modelValue.picList || [];
    const slidesPerView = props.modelValue.number > 0 ? props.modelValue.number : 3;
    
    // 如果原始列表为空，直接返回
    if (originalList.length === 0) {
        return originalList;
    }
    
    // 计算需要的最小图片数量（至少是 slidesPerView * 2，确保循环模式正常工作）
    const minRequired = slidesPerView * 2;
    
    // 如果图片数量已经足够，直接返回原列表
    if (originalList.length >= minRequired) {
        return originalList;
    }
    
    // 如果图片数量不足，复制图片来补充
    const extendedList = [...originalList];
    let currentIndex = 0;
    
    // 循环复制，直到达到最小数量要求
    while (extendedList.length < minRequired) {
        extendedList.push({ ...originalList[currentIndex % originalList.length] });
        currentIndex++;
    }
    
    return extendedList;
});

// 更新 swiper 配置的函数
const updateSwiperOption = () => {
    const slidesPerView = props.modelValue.number > 0 ? props.modelValue.number : 3;
    const slidesPerGroup = props.modelValue.number > 0 ? props.modelValue.number : 3;
    
    // 始终启用循环模式，因为我们已经通过 extendedPicList 确保了足够的图片数量
    swiperOption.value = {
        ...swiperOption.value,
        slidesPerGroup: slidesPerGroup,
        slidesPerView: slidesPerView,
        loop: true, // 始终启用循环
        loopAdditionalSlides: slidesPerView, // 添加额外的幻灯片以确保循环正常工作
        loopedSlidesLimit: false // 不限制循环的幻灯片数量
    };
};

const swiperOption = ref<any>({
    // Swiper的配置选项
    // 根据实际需求进行配置
    // 例如：autoplay, loop, navigation等
    autoplay: {
        delay: 5000,
        disableOnInteraction: false, //用户操作swiper之后，是否禁止autoplay
        pauseOnMouseEnter: true //鼠标置于swiper是否时暂停自动切换
    },
    spaceBetween: 30,
    slidesPerGroup: props.modelValue.number > 0 ? props.modelValue.number : 3,
    slidesPerView: props.modelValue.number > 0 ? props.modelValue.number : 3,
    navigation: true,
    effect: "slide",
    loop: false // 初始值，会在 updateSwiperOption 中更新
});

// 初始化时更新配置
updateSwiperOption();

watch(
    () => props.modelValue.number,
    () => {
        updateSwiperOption();
    },
    { deep: true }
);

// 监听图片列表变化，动态更新循环模式
watch(
    () => props.modelValue.picList,
    (newList) => {
        updateSwiperOption();
    },
    { deep: true }
);
</script>
<style lang="less" scoped>
.banner {
    position: relative;
    margin-top: 32px;
    // 跟随全站 .container 限宽（旧版 1190），勿写死 1264
    width: 100%;
    height: 195px;
    .inner {
        position: relative;
        width: 100%;
        height: 195px;
        border-radius: 8px;
        :deep(.swiper) {
            width: 100%;
            height: 195px;
            margin-right: 10px;
            border-radius: 8px;
            overflow: hidden;
            user-select: none;
            -webkit-user-select: none;
            -moz-user-select: none;
            -ms-user-select: none;
        }
        .slider_item {
            display: flex;
            justify-content: center;
            height: 195px;
            min-height: 1px;
            user-select: none;
            -webkit-user-select: none;
            -moz-user-select: none;
            -ms-user-select: none;
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
                user-select: none;
                -webkit-user-select: none;
                -moz-user-select: none;
                -ms-user-select: none;
                .item-img {
                    display: block;
                    width: 100%;
                    height: 100%;
                    transition: opacity 0.2s;
                    background-color: #eeeeee;
                    user-select: none;
                    -webkit-user-select: none;
                    -moz-user-select: none;
                    -ms-user-select: none;
                    pointer-events: none;
                }
            }
            .item__ad_lk-2 {
                display: inline-block;
                width: 100%;
                height: 100%;
                position: relative;
                border-radius: 8px;
                overflow: hidden;
                user-select: none;
                -webkit-user-select: none;
                -moz-user-select: none;
                -ms-user-select: none;
                .item-img-2 {
                    display: block;
                    width: 100%;
                    height: 100%;
                    transition: opacity 0.2s;
                    background-color: #eeeeee;
                    user-select: none;
                    -webkit-user-select: none;
                    -moz-user-select: none;
                    -ms-user-select: none;
                    pointer-events: none;
                }
            }
            .item__ad_lk-3 {
                display: inline-block;
                width: 100%;
                height: 100%;
                position: relative;
                border-radius: 8px;
                overflow: hidden;
                user-select: none;
                -webkit-user-select: none;
                -moz-user-select: none;
                -ms-user-select: none;
                .item-img-3 {
                    display: block;
                    width: 100%;
                    height: 100%;
                    transition: opacity 0.2s;
                    background-color: #eeeeee;
                    user-select: none;
                    -webkit-user-select: none;
                    -moz-user-select: none;
                    -ms-user-select: none;
                    pointer-events: none;
                }
            }
        }
    }
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
