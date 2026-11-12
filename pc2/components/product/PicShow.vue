<template>
    <div class="goods-image" ref="goodsImageRef">
        <!-- 大图（放大镜预览） -->
        <template v-if="mainImage">
            <div v-show="show" class="large" :style="largeStyle"></div>
        </template>

        <!-- 中图 -->
        <div class="middle" :class="{ video: images?.videoUrl }">
            <template v-if="mainImage">
                <template v-if="!images?.videoUrl">
                    <img
                        style="object-fit: contain"
                        v-sold-out="{
                            productStock: product.productStock,
                            productStatus: product.productStatus,
                            productType: product.productType,
                            deliveryMode: product.deliveryMode,
                            deliveryType: product.deliveryType,
                            uiVersion
                        }"
                        ref="target"
                        @mouseenter="onImageEnter"
                        @mouseleave="onImageLeave"
                        :src="mainImage"
                        :alt="product.productName || ''"
                    />
                </template>
                <template v-else>
                    <img
                        style="object-fit: contain"
                        v-sold-out="{
                            productStock: product.productStock,
                            productStatus: product.productStatus,
                            productType: product.productType,
                            deliveryMode: product.deliveryMode,
                            deliveryType: product.deliveryType,
                            uiVersion
                        }"
                        @click="$emit('play')"
                        :src="images.videoCover ? imageFormat(images.videoCover) : mainImage"
                        :alt="product.productName || ''"
                    />
                </template>
            </template>
            <template v-if="images?.videoUrl">
                <div class="video-icon" @click="$emit('play')"></div>
            </template>
        </div>
    </div>
</template>
<script lang="ts" setup>
import { ref, watch, reactive, computed, inject, onMounted, onBeforeUnmount, type ComputedRef } from "vue";
import { useMouseInElement } from "@vueuse/core";
import { imageFormat, productImageLarge } from "@/utils/format";

const props = defineProps<{
    images: any;
    product: any;
    disableZoom?: boolean;
}>();

// 商品详情当前 UI 版本（由页面通过 provide 下发），用于售罄遮罩等按新版样式渲染
const detailUiVersion = inject<ComputedRef<string> | undefined>("detailUiVersion", undefined);
const uiVersion = computed(() => detailUiVersion?.value ?? "old");

const emit = defineEmits(["play"]);

// 主图与放大镜共用大图地址（picLarge 优先，避免落到带 resize 的 picUrl）
const mainImage = computed(() => productImageLarge(props.images));

// 放大镜缩放倍数 & 预览框与大图之间的间距
const ZOOM = 2;
const PREVIEW_GAP = 80;
// 预览框默认尺寸（右侧空间不足时会自动收缩）
const PREVIEW_SIZE = 430;

// 1. 是否显示遮罩和大图
const show = ref(false);

// 鼠标是否真正悬浮在大图上（基于原生 mouseenter/mouseleave）。
// useMouseInElement 仅根据坐标判断 isOutside，弹窗/遮罩覆盖大图时坐标依旧命中，
// 会导致弹窗底下的放大预览被误触发；用原生事件门控可避免穿透（被遮罩覆盖时不会触发 enter）。
const isHovering = ref(false);
const onImageEnter = () => {
    if (props.disableZoom) return;
    isHovering.value = true;
};
const onImageLeave = () => {
    isHovering.value = false;
    show.value = false;
};

// 距离视口右侧预留的安全间距（避免贴边/出现滚动条）
const SAFE_GAP = 8;

// 2. 大图（中图）的实时尺寸 & 右侧可用空间，用于响应式计算预览框大小与位置
const goodsImageRef = ref<HTMLElement | null>(null);
const imgSize = ref(0);
const availableRight = ref(0);

const measure = () => {
    const el = goodsImageRef.value;
    if (!el) return;
    imgSize.value = el.clientWidth;
    // 大图右侧到视口右边缘的可用宽度
    // 注意：window.innerWidth 包含竖直滚动条宽度，会高估可用空间导致预览框溢出产生横向滚动条，
    // 这里使用 documentElement.clientWidth（不含滚动条）来计算真正可用的宽度。
    const viewportWidth = document.documentElement.clientWidth || window.innerWidth;
    availableRight.value = viewportWidth - el.getBoundingClientRect().right;
};

let resizeObserver: ResizeObserver | null = null;
onMounted(() => {
    measure();
    if (goodsImageRef.value) {
        resizeObserver = new ResizeObserver(() => measure());
        resizeObserver.observe(goodsImageRef.value);
    }
    // 大图被 max-width 限制时尺寸不变，但视口右侧空间会变化，需额外监听窗口
    window.addEventListener("resize", measure);
});
onBeforeUnmount(() => {
    resizeObserver?.disconnect();
    resizeObserver = null;
    window.removeEventListener("resize", measure);
});

// 预览框与大图之间的间距：右侧空间不足时优先收窄间距（先压间距，再缩预览框）
const previewGap = computed(() => {
    const gapForFullSize = availableRight.value - PREVIEW_SIZE - SAFE_GAP;
    return Math.max(0, Math.min(PREVIEW_GAP, gapForFullSize));
});

// 预览框尺寸：默认固定为 PREVIEW_SIZE，间距压到 0 后仍不足时再收缩预览框，避免溢出产生滚动条
const previewSize = computed(() => {
    const maxSize = Math.max(0, availableRight.value - previewGap.value - SAFE_GAP);
    return Math.min(PREVIEW_SIZE, maxSize);
});

// 3. 大图的背景坐标(样式)
const largePosition = reactive({
    backgroundPositionX: "0px",
    backgroundPositionY: "0px"
});

// 预览框样式：尺寸、位置、背景图与缩放均随大图尺寸与可用空间自适应
const largeStyle = computed(() => {
    const url = mainImage.value;
    return {
        width: `${previewSize.value}px`,
        height: `${previewSize.value}px`,
        left: `calc(100% + ${previewGap.value}px)`,
        backgroundImage: `url(${url})`,
        backgroundSize: `${imgSize.value * ZOOM}px ${imgSize.value * ZOOM}px`,
        backgroundPositionX: largePosition.backgroundPositionX,
        backgroundPositionY: largePosition.backgroundPositionY
    };
});

// 4. 获取监听元素的实例对象, 使用useMouseInElement函数进行获取坐标
const target = ref(null);
const { elementX, elementY, isOutside } = useMouseInElement(target);

watch(
    () => props.disableZoom,
    (val) => {
        if (val) {
            isHovering.value = false;
            show.value = false;
        }
    }
);

// 5. 监听元素坐标值的变化，根据当前大图尺寸计算放大镜取景位置
watch([elementX, elementY, isOutside], () => {
    show.value = isHovering.value && !isOutside.value && !props.disableZoom;
    const size = imgSize.value;
    if (!size) return;

    // 取景框（lens）在大图上的尺寸，与预览框大小对应
    const lens = previewSize.value / ZOOM;
    let lensX = elementX.value - lens / 2;
    let lensY = elementY.value - lens / 2;
    lensX = Math.max(0, Math.min(size - lens, lensX));
    lensY = Math.max(0, Math.min(size - lens, lensY));

    largePosition.backgroundPositionX = `${-lensX * ZOOM}px`;
    largePosition.backgroundPositionY = `${-lensY * ZOOM}px`;
});
</script>

<style lang="less" scoped>
.goods-image {
    position: relative;
    display: flex;
    // z-index: 6;
    width: 100%;
    height: 100%;

    .large {
        position: absolute;
        top: 10px;
        box-shadow: 0 0 10px rgba(0, 0, 0, 0.1);
        background-repeat: no-repeat;
        background-color: #f8f8f8;
        z-index: 999;
        border-radius: 12px;
        overflow: hidden;
    }

    .middle {
        width: 100%;
        height: 100%;
        background: #f5f5f5;
        position: relative;
        cursor: move;
        display: flex;
        align-items: center;
        justify-content: center;

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
            width: 100%;
            height: 100%;
            object-fit: contain;
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
.soldOut {
    position: absolute;
    top: 0;
    left: 0;
    bottom: 0;
    right: 0;
    margin: auto;
    width: 70%;
    height: 70%;
    background-color: rgba(0, 0, 0, 0.5);
    color: #fff;
    font-size: 20px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 100%;
    z-index: 7;
    pointer-events: auto;
    cursor: move;
}
</style>
