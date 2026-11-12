<template>
    <div class="goods-gallery flex">
        <div class="slider" :class="sliderMaskClass" ref="sliderRef">
            <div class="slider-content">
                <div class="slider-wrapper">
                    <template v-for="(item, index) in list" :key="index">
                        <div class="slider-item" ref="sliderItemRefs" @mouseover="handleMouseOver(index)">
                            <img
                                :src="item.videoCover ? imageFormat(item.videoCover) : productImage(item)"
                                :class="isThumbActive(index) ? 'cur' : ''"
                                :alt="productName || ''"
                            />
                            <template v-if="item.videoUrl">
                                <div class="video-icon"></div>
                            </template>
                        </div>
                    </template>
                </div>
            </div>
        </div>
        <div class="gallery-container">
            <div class="gallery" ref="galleryRef" :style="{ '--switch-offset': switchOffset }">
                <PicShow @play="showVideo = true" v-show="!showVideo" :product="product" :images="activeImage" :disableZoom="disableZoom" />
                <template v-if="videoData.videoUrl">
                    <productVideo
                        :useCoverBg="true"
                        :poster="videoData.videoCover || pickPic(list[0])"
                        v-show="showVideo"
                        :showVideo="showVideo"
                        :src="videoData.videoUrl"
                        :type="`video/${videoData.format ? videoData.format : 'mp4'}`"
                        class="video-box"
                        @ended="showVideo = false"
                    ></productVideo>
                </template>
                <template v-if="list.length > 1">
                    <button class="switch-btn switch-prev" :class="{ disabled: imgIndex <= 0 }" @click="prevImage">
                        <i class="iconfont-pc icon-jiantou_zuo"></i>
                    </button>
                    <button class="switch-btn switch-next" :class="{ disabled: imgIndex >= list.length - 1 }" @click="nextImage">
                        <i class="iconfont-pc icon-jiantou_you"></i>
                    </button>
                </template>
            </div>
        </div>
    </div>
</template>

<script setup lang="ts">
import PicShow from "~/components/product/PicShow.vue";
import { productVideo } from "~/components/product";
import { imageFormat, pickPic, productImage } from "@/utils/format";
import type { PicList, VideoList } from "~/types/product/product.d";

const props = defineProps({
    picList: {
        type: Array as PropType<PicList[]>,
        default: []
    },
    videoList: {
        type: Array as PropType<VideoList[]>,
        default: []
    },
    product: {
        type: Object,
        default: () => ({})
    },
    disableZoom: {
        type: Boolean,
        default: false
    },
    productName: {
        type: String,
        default: ""
    }
});

const videoData = computed(() => {
    return props.videoList[0] ? props.videoList[0] : ({} as VideoList);
});

const list = computed(() => {
    return videoData.value?.videoUrl
        ? [
              {
                  ...videoData.value,
                  picUrl: pickPic(props.picList[0]),
                  picLarge: videoData.value.videoCover || props.picList[0]?.picLarge || props.picList[0]?.picUrl || ""
              },
              ...props.picList
          ]
        : props.picList;
});

const showVideo = ref(false);

const imgIndex = ref(0);

const sliderRef = ref<HTMLElement | null>(null);
const sliderItemRefs = ref<HTMLElement[]>([]);

// 将当前选中的缩略图滚动到可视区域内
const scrollThumbIntoView = () => {
    const container = sliderRef.value;
    const target = sliderItemRefs.value[imgIndex.value];
    if (!container || !target) return;
    const cRect = container.getBoundingClientRect();
    const tRect = target.getBoundingClientRect();
    if (tRect.top < cRect.top) {
        container.scrollBy({ top: tRect.top - cRect.top, behavior: "smooth" });
    } else if (tRect.bottom > cRect.bottom) {
        container.scrollBy({ top: tRect.bottom - cRect.bottom, behavior: "smooth" });
    }
};

watch(imgIndex, () => {
    nextTick(scrollThumbIntoView);
});

// 选中第一张为 start 样式，最后一张为 end 样式，其余为 middle
const sliderMaskClass = computed(() => {
    if (imgIndex.value <= 0) return "start";
    if (imgIndex.value >= list.value.length - 1) return "end";
    return "middle";
});

const getShowVideo = () => {
    showVideo.value = list.value[imgIndex.value]?.videoUrl ? true : false;
};

getShowVideo();

// 规格图生效时主图区展示图片，需退出视频态（视频随之暂停）
const { activeImage, isThumbActive, exitSpecPic } = useSpecPicGallery({
    list,
    imgIndex,
    onApply: () => {
        showVideo.value = false;
    }
});

const handleMouseOver = (index: number) => {
    exitSpecPic();
    imgIndex.value = index;
    getShowVideo();
};

// 切换大图：上一张
const prevImage = () => {
    exitSpecPic();
    if (imgIndex.value <= 0) return;
    imgIndex.value--;
    getShowVideo();
};

// 切换大图：下一张
const nextImage = () => {
    exitSpecPic();
    if (imgIndex.value >= list.value.length - 1) return;
    imgIndex.value++;
    getShowVideo();
};

// 根据大图两侧留白，动态计算切换按钮位置：
// 留白足够时按钮移到图片外侧（最远 -64px），留白不足时按钮叠加在图片内（最近 16px）。
const galleryRef = ref<HTMLElement | null>(null);
const switchOffset = ref("16px");

const updateSwitchOffset = () => {
    const el = galleryRef.value;
    const parent = el?.parentElement;
    if (!el || !parent) return;
    // 图片居中后右侧的留白宽度（与左侧留白对称）
    const margin = parent.getBoundingClientRect().right - el.getBoundingClientRect().right;
    let offset = 16 - 1.25 * margin;
    offset = Math.max(-64, Math.min(16, offset));
    switchOffset.value = `${offset}px`;
};

let resizeObserver: ResizeObserver | null = null;
onMounted(() => {
    updateSwitchOffset();
    if (galleryRef.value?.parentElement) {
        resizeObserver = new ResizeObserver(() => updateSwitchOffset());
        resizeObserver.observe(galleryRef.value.parentElement);
    }
});
onBeforeUnmount(() => {
    resizeObserver?.disconnect();
    resizeObserver = null;
});
</script>

<style lang="less" scoped>
.goods-gallery {
    position: relative;
    display: flex;
    margin-bottom: 10px;
    width: 100%;

    &:hover .switch-btn {
        opacity: 1;

        &.disabled {
            opacity: 0.4;
        }
    }
}

.gallery-container {
    flex: 1;
    min-width: 0;
    margin-left: 92px;
    display: flex;
    justify-content: center;
    // 用宽度上限保证 4:3 高度不超过 620，避免 max-height 压扁后再被图片撑开
    width: min(100%, calc(620px * 4 / 3));
    aspect-ratio: 4 / 3;
}

.gallery {
    display: flex;
    flex-direction: column;
    justify-content: center;
    width: 100%;
    height: 100%;
    border-radius: 12px;
    position: relative;

    .video-box {
        width: 100%;
        height: 100%;
        border-radius: 12px;
        overflow: hidden;
    }

    img {
        width: 100%;
        height: 100%;
        font-size: 5px;
    }

    // 主图占满预留盒，不随图片固有尺寸改变布局，避免加载完成时页面跳动
    :deep(.goods-image) {
        display: flex;
        justify-content: center;
        width: 100%;
        height: 100%;
        max-width: 100%;
        max-height: 100%;
    }

    :deep(.middle) {
        width: 100%;
        height: 100%;
        border-radius: 12px;
        overflow: hidden;
    }
}

.switch-btn {
    position: absolute;
    top: 50%;
    transform: translateY(-50%);
    z-index: 10;
    width: 52px;
    height: 52px;
    border: 1px solid #eee;
    border-radius: 50%;
    background: #fff;
    color: #000;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    opacity: 0;
    transition: opacity 0.3s, color 0.3s, box-shadow 0.3s;

    .iconfont-pc {
        font-size: 42px;
        line-height: 1;
    }

    &:hover {
        color: var(--general);
    }

    &.disabled {
        cursor: not-allowed;
        pointer-events: none;
    }

    // 位置由 JS 根据大图两侧留白实时计算（见 updateSwitchOffset）
    &.switch-prev {
        left: var(--switch-offset, 16px);
    }

    &.switch-next {
        right: var(--switch-offset, 16px);
    }
}

.slider {
    position: absolute;
    top: 0;
    width: 72px;
    height: 100%;
    overflow: auto;
    display: flex;
    margin-right: 16px;
    flex-direction: column;
    align-items: center;
    &::-webkit-scrollbar {
        display: none;
    }
    &::-webkit-scrollbar-thumb {
        display: none;
    }
    &::-webkit-scrollbar-track {
        display: none;
    }
    &.start {
        mask: linear-gradient(rgb(255, 255, 255) 0%, rgb(255, 255, 255) 0%, rgb(255, 255, 255) 89%, rgba(255, 255, 255, 0) 100%);
    }
    &.middle {
        mask: linear-gradient(rgba(255, 255, 255, 0) 0%, rgba(255, 255, 255, 0.925) 10%, rgb(255, 255, 255) 89%, rgba(255, 255, 255, 0) 100%);
    }
    &.end {
        mask: linear-gradient(rgba(255, 255, 255, 0) 0%, rgb(255, 255, 255) 9%, rgb(255, 255, 255) 100%);
    }
}

.slider-wrapper {
    display: flex;
    flex-direction: column;
    row-gap: 12px;
}

.slider-item {
    width: 72px;
    height: 72px;
    position: relative;
    display: flex;
    align-items: center;
    justify-content: center;

    img {
        border: 2px solid transparent;
        width: 72px;
        height: 72px;
        object-fit: cover;
        cursor: pointer;
        background-color: rgba(31, 31, 51, 0.04);
        border-radius: 12px;
        box-sizing: border-box;
    }

    .cur {
        border-color: var(--general);
    }

    .video-icon {
        position: absolute;
        top: 50%;
        left: 50%;
        width: 28px;
        height: 28px;
        transform: translate(-50%, -50%);
        background-image: url("~/assets/images/product/video-playing.png");
        background-size: cover;
    }
}
</style>
