<template>
    <div class="goods-image">
        <!-- 大图 -->
        <template v-if="mainImage && !showEnquiry">
            <div v-show="show" class="large" :style="[{ backgroundImage: `url(${mainImage})` }, largePosition]"></div>
        </template>

        <!-- 中图 -->
        <div class="middle" :class="{ video: images.videoUrl }">
            <template v-if="mainImage">
                <template v-if="!images.videoUrl">
                    <img ref="target" @mouseenter="onImageEnter" @mouseleave="onImageLeave" :src="mainImage" :alt="productName || ''" />
                </template>
                <template v-else>
                    <img @click="$emit('play')" :src="images.videoCover ? imageFormat(images.videoCover) : mainImage" :alt="productName || ''"
                /></template>
            </template>
            <template v-if="images.videoUrl">
                <div class="video-icon" @click="$emit('play')"></div>
            </template>
        </div>
    </div>
</template>
<script lang="ts" setup>
import { ref, watch, reactive, computed } from "vue";
import { useMouseInElement } from "@vueuse/core";
import { imageFormat, productImageLarge } from "@/utils/format";

const props = defineProps<{
    images: any;
    dialogVisible?: boolean;
    showEnquiry: boolean;
    productName?: string;
}>();

const emit = defineEmits(["play"]);

// 主图与放大镜共用大图地址（picLarge 优先，避免落到带 resize 的 picUrl）
const mainImage = computed(() => productImageLarge(props.images));

// 1. 是否显示遮罩和大图
const show = ref(false);

// 鼠标是否真正悬浮在大图上（基于原生 mouseenter/mouseleave）。
// useMouseInElement 仅根据坐标判断 isOutside，弹窗/遮罩覆盖大图时坐标依旧命中，
// 会导致弹窗底下的放大预览被误触发；用原生事件门控可避免穿透（被遮罩覆盖时不会触发 enter）。
const isHovering = ref(false);
const onImageEnter = () => {
    isHovering.value = true;
};
const onImageLeave = () => {
    isHovering.value = false;
    show.value = false;
};

// 2. 遮罩的坐标(样式)
// let layerPosition = reactive({
//   top: '0',
//   left: '0'
// })

// 3. 大图的背景坐标(样式)
const largePosition = reactive({
    backgroundPositionX: "0",
    backgroundPositionY: "0"
});

// 4. 获取监听元素的实例对象, 使用useMouseInElement函数进行获取坐标
const target = ref(null);
const { elementX, elementY, isOutside } = useMouseInElement(target);

// 5. 监听元素坐标值的变化
watch([elementX, elementY, isOutside], () => {
    show.value = isHovering.value && !isOutside.value;
    const position = { X: 0, Y: 0 };

    if (elementX.value < 100) position.X = 0;
    else if (elementX.value > 250) position.X = 160;
    else position.X = elementX.value - 100;

    if (elementY.value < 100) position.Y = 0;
    else if (elementY.value > 250) position.Y = 160;
    else position.Y = elementY.value - 100;

    // layerPosition.left = position.X + 'px'
    // layerPosition.top = position.Y + 'px'
    largePosition.backgroundPositionX = -2 * position.X + "px";
    largePosition.backgroundPositionY = -2 * position.Y + "px";
});
</script>

<style lang="less" scoped>
.goods-image {
    position: relative;
    display: flex;
    // z-index: 6;

    .large {
        position: absolute;
        top: 0;
        left: 635px;
        width: 500px;
        height: 500px;
        box-shadow: 0 0 10px rgba(0, 0, 0, 0.1);
        background-repeat: no-repeat;
        background-size: 800px 800px;
        background-color: #f8f8f8;
        z-index: 1999;
    }
    .large.disabled {
        pointer-events: none;
        display: none;
    }

    .middle {
        width: 624px;
        height: 624px;
        background: #f5f5f5;
        position: relative;
        cursor: move;

        &.video {
            cursor: pointer;
        }

        .video-icon {
            position: absolute;
            top: 50%;
            left: 50%;
            width: 68px;
            height: 68px;
            transform: translate(-50%, -50%);
            background-image: url("~/assets/images/product/video-playing.png");
            background-size: cover;
        }

        img {
            width: 624px;
            height: 624px;
            max-width: 100%;
            max-height: 100%;
            object-fit: contain;
            object-position: center;
        }

        .layer {
            width: 160px;
            height: 160px;
            background: rgba(0, 0, 0, 0.2);
            left: 0;
            top: 0;
            position: absolute;
        }
    }
}
</style>
