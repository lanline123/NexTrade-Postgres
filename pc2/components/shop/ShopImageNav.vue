<template>
    <TotalContainer :module="module.frame">
        <XRowX v-if="module.navType == 1" :module="module"></XRowX>
        <CarouselNav v-if="module.navType == 2" :module="module"></CarouselNav>
    </TotalContainer>
</template>
<script setup lang="ts">
import { ref } from "vue";
import TotalContainer from "~/components/shop/src/TotalContainer.vue";
import XRowX from "~/components/shop/src/XRowX.vue";
import CarouselNav from "~/components/shop/src/CarouselNav.vue";

const props = defineProps({
    module: { type: Object, default: {} }
});

const swiperOption = ref<any>({
    autoplay: {
        delay: 3022200,
        disableOnInteraction: false, //用户操作swiper之后，是否禁止autoplay
        pauseOnMouseEnter: true //鼠标置于swiper是否时暂停自动切换
    },
    pagination: {
        clickable: true
    }
});
// 计算属性，生成用于显示的项目列表，包括可能的占位符
const displayItems = computed(() => {
    const totalSlots = props.module.colNum * props.module.rowNum; // 总格子数
    const itemCount = Math.min(props.module.picList.length, totalSlots); // 实际显示的项目数量

    // 如果项目不够填满所有格子，用空对象补足
    const paddedItems = props.module.picList.concat(new Array(totalSlots - itemCount).fill({}));

    return paddedItems.map((item: any, index: any) => ({
        ...item, // 保留原项目数据
        // 示例用的虚拟图片URL和文本，实际应用中应从props.items中获取
        imageUrl: item.imageUrl || `https://via.placeholder.com/150?text=Image${index + 1}`,
        text: item.text || `Text for item ${index + 1}`
    }));
});
const placeholderItem: any = ref(null);
const adjustedPicList = computed(() => {
    let pageSize = props.module.colNum * props.module.rowNum; // 根据type确定每页数量
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
.base-class {
    display: flex;
    flex-direction: column;
    align-items: center;
    box-sizing: border-box;

    .image-list {
        width: 100%;
        padding: 10px;
        box-sizing: border-box;
    }

    .image-list-1 {
        display: flex;
        flex-direction: row;
        flex-wrap: wrap;
        box-sizing: border-box;
        width: 100%;
        .image {
            box-sizing: border-box;
            display: flex;
            flex-direction: column;
            align-items: center;
            gap: 4px;

            .title-sub {
                text-align: center;
                height: 16px;
                line-height: 16px;
                font-size: 12px;
                overflow: hidden; /* 确保文字超出部分隐藏 */
            }
        }
        .slider_wrapper {
            width: 100%;
            padding: 20px 0;
            box-sizing: border-box;

            .image-ad-item {
                display: flex;
                flex-direction: row;
                flex-wrap: wrap;
                box-sizing: border-box;

                .item-content {
                    box-sizing: border-box;
                    display: flex;
                    flex-direction: column;
                    align-items: center;

                    .title-sub {
                        text-align: center;
                        height: 26px;
                        line-height: 26px;
                        font-size: 12px;
                        overflow: hidden; /* 确保文字超出部分隐藏 */
                    }
                }
            }
        }
    }
}

.image-box {
    flex: 1;

    cursor: pointer;
}

.image-box-radius {
    border-radius: 8px;
}

.image-list :deep(.swiper) {
    padding-bottom: 30px;
}

.image-list :deep(.swiper-button-prev),
.image-list :deep(.swiper-button-next) {
    z-index: 2;
    border: none;
    outline: none;
    transition: background-color 0.2s ease;
}

.image-list :deep(.swiper-button-prev) {
    left: 10px;
    border-top-right-radius: 2px;
    border-bottom-right-radius: 2px;
}

.image-list :deep(.swiper-button-next) {
    right: 10px;
    border-top-left-radius: 2px;
    border-bottom-left-radius: 2px;
}

.image-list :deep(.swiper-button-prev:after),
.image-list :deep(.swiper-button-next:after) {
    color: #e5e5e5;
    font-size: 24px;
    font-weight: bolder;
}

.image-list :deep(.swiper-pagination) {
    padding-left: 25px;
    padding-bottom: 10px;
    text-align: center;
}

.image-list :deep(.swiper-pagination-bullet) {
    position: relative;
    display: inline-block;
    transition: background 0.2s ease;
    background-color: #fff;
}

.image-list-bt-1 :deep(.swiper-pagination-bullet) {
    width: 12px;
    height: 4px;
    border-radius: 2px;
    margin: 0 1px;
}

.image-list-bt-1 :deep(.swiper-pagination-bullet-active) {
    width: 12px;
    height: 4px;
    border-radius: 2px;
    margin: 0 1px;
}

.image-list-bt-2 :deep(.swiper-pagination-bullet) {
    width: 12px;
    height: 12px;
    border-radius: 50%;
    margin: 0 3px;
}

.image-list-bt-2 :deep(.swiper-pagination-bullet-active) {
    width: 12px;
    height: 12px;
    border-radius: 50%;
    margin: 0 3px;
}

.image-list-bt-3 :deep(.swiper-pagination) {
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

.image-list-bt-3 :deep(.swiper-pagination-bullet) {
    width: 12px;
    height: 12px;
    opacity: 1;
    border-radius: 50%;
    margin: 0 3px;
}

.image-list-bt-3 :deep(.swiper-pagination-bullet-active) {
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
