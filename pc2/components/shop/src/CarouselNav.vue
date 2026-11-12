<template>
    <div class="carousel-list" :class="['carousel-list-bt-' + module.picPageType]">
        <swiper class="slider_wrapper" v-model:swiperOption="swiperOption" :itemList="adjustedPicList">
            <template v-slot:default="{ item }">
                <div class="image-ad-item">
                    <template v-for="img in item">
                        <NuxtLink
                            target="_blank"
                            :to="urlFormat(img.picLink)"
                            class="image-container"
                            :style="{ padding: module.imgPadding + 'px', width: 100 / module.rowNum + '%' }"
                        >
                            <el-image
                                v-if="module.navStyle != 3"
                                :class="[module.radioStyle == 2 ? 'image-box-radius' : '']"
                                class="image-box"
                                :src="imageFormat(img.picUrl)"
                            ></el-image>
                            <div v-if="module.navStyle != 2" class="pic-title">{{ img.picTitle }}</div>
                        </NuxtLink>
                    </template>
                </div>
            </template>
        </swiper>
    </div>
</template>
<script setup lang="ts">
import { imageFormat, urlFormat } from "~/utils/format";
import { ref } from "vue";

const props = defineProps({
    module: { type: Object, default: {} }
});
const swiperOption = ref<any>({
    autoplay: {
        delay: 3000,
        disableOnInteraction: false, //用户操作swiper之后，是否禁止autoplay
        pauseOnMouseEnter: true //鼠标置于swiper是否时暂停自动切换
    },
    pagination: {
        clickable: true
    }
});
const placeholderItem: any = ref(null);
const adjustedPicList = computed(() => {
    let pageSize = props.module.rowNum * props.module.colNum; // 根据type确定每页数量
    let totalItems = props.module.picList?.length; // 总图片数量
    let pageCount = Math.ceil(props.module.picList?.length / pageSize); // 计算总页数
    let adjustedLists = [];

    for (let i = 0; i < pageCount; i++) {
        let page = []; // 当前页的图片列表
        for (let j = 0; j < pageSize; j++) {
            // 计算当前页实际应该显示的图片索引
            let index = i * pageSize + j;
            if (index < totalItems) {
                // 添加真实图片
                page.push(props.module.picList[index]);
            } else {
                // 如果图片已经放完，但本页未满，则添加占位符
                if (!placeholderItem.value) {
                    // 首次使用时创建一个占位符对象
                    placeholderItem.value = { __isPlaceholder: true };
                }
                page.push(placeholderItem.value);
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

    .slider_wrapper {
        width: 100%;
        .image-ad-item {
            width: 100%;
            display: flex;
            flex-direction: row;
            flex-wrap: wrap;

            .image-container {
                text-align: center; /* 图片居中显示 */
                box-sizing: border-box;
                display: flex;
                flex-direction: column;
                gap: 6px;
                .pic-title {
                    height: 20px;
                    font-size: 12px;
                    line-height: 1.5;
                    white-space: nowrap; /* 不换行 */
                    overflow: hidden; /* 隐藏超出容器的内容 */
                    text-overflow: ellipsis; /* 超出部分用省略号表示 */
                }
            }
        }
    }
}

.image-box {
    flex: 1;
}

.image-box-radius {
    border-radius: 8px;
}

.carousel-list :deep(.swiper) {
    padding-bottom: 30px;
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
    text-align: center;
}

.carousel-list :deep(.swiper-pagination-bullet) {
    position: relative;
    display: inline-block;
    transition: background 0.2s ease;
    background-color: #fff;
}

.carousel-list-bt-1 :deep(.swiper-pagination-bullet) {
    width: 12px;
    height: 4px;
    border-radius: 2px;
    margin: 0 1px;
}

.carousel-list-bt-1 :deep(.swiper-pagination-bullet-active) {
    width: 12px;
    height: 4px;
    border-radius: 2px;
    margin: 0 1px;
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
