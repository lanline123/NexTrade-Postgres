<template>
    <div class="gallery">
        <PicShow @play="showVideo = true" v-show="!showVideo" :product="product" :images="activeImage" :disableZoom="disableZoom" />
        <template v-if="videoData.videoUrl">
            <productVideo
                v-show="showVideo"
                :showVideo="showVideo"
                :src="videoData.videoUrl"
                :type="`video/${videoData.format ? videoData.format : 'mp4'}`"
                class="video-box"
                @ended="showVideo = false"
            ></productVideo>
        </template>
    </div>
    <div class="slider">
        <button class="prev" @click="prevSlide"></button>
        <div class="slider-content">
            <div class="slider-wrapper flex" :style="{ transform: `translateX(${offset}px)` }">
                <template v-for="(item, index) in list" :key="index">
                    <div class="slider-item">
                        <img
                            :src="item.videoCover ? imageFormat(item.videoCover) : productImage(item)"
                            :class="isThumbActive(index) ? 'cur' : ''"
                            :alt="product.productName || ''"
                            @mouseover="handleMouseOver(index)"
                        />
                        <template v-if="item.videoUrl">
                            <div class="video-icon"></div>
                        </template>
                    </div>
                </template>
            </div>
        </div>
        <button class="next" @click="nextSlide"></button>
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

const offset = ref(0);

const showVideo = ref(false);

const imgIndex = ref(0);

const num = ref(0);

watchEffect(() => {
    if (list.value.length > 4) {
        num.value = list.value.length - 4;
    } else {
        num.value = 0;
    }
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

const lengthNum = () => {
    let length = list.value.length;
    let num = length / 4;
    if (num - Math.floor(num) !== 0) {
        num = Math.ceil(num);
    }
    let leg = num * 77.5;
    return leg;
};

const prevSlide = () => {
    // if (offset.value != 0) offset.value += 77.5;
    if (num.value >= list.value.length - 4) return;
    offset.value += 77.5;
    num.value++;
};

const nextSlide = () => {
    if (num.value <= 0) return;
    num.value--;
    offset.value -= 77.5;

    // if (offset.value != -lengthNum() - -77.5) offset.value -= 77.5;
};
</script>

<style lang="less" scoped>
.gallery {
    width: 358px;
    height: 358px;
    margin: 0 auto 0;
    margin-bottom: 10px;
    border: 1px solid #eee;

    .video-box {
        width: 100%;
        height: 100%;
    }

    img {
        width: 358px;
        height: 358px;
        font-size: 5px;
    }
}
.slider {
    width: 360px;
    position: relative;
    display: flex;
    justify-content: space-between;

    .slider-content {
        overflow: hidden;
        width: 310px;
    }
}

.slider-wrapper {
    display: flex;
    transition: transform 0.5s;
    width: 2000px;
    overflow: hidden;
}

.slider-item {
    width: 77.5px;
    position: relative;

    /* 假设每张图片宽度为400px */
    img {
        border: 1px solid #e7e7e7;
        height: 67.5px;
        width: 67.5px;
        object-fit: contain;
    }

    .cur {
        border-color: var(--general);
    }

    .video-icon {
        position: absolute;
        top: 50%;
        left: 50%;
        width: 32px;
        height: 32px;
        transform: translate(-50%, -50%);
        background-image: url("~/assets/images/product/video-playing.png");
        background-size: cover;
    }
}

.prev,
.next {
    background: url("/assets/images/product/sli_btn.png") no-repeat scroll 5px center;
    cursor: pointer;
    height: 70px;
    width: 18px;
}

.prev {
    border-left: 0;
}

.prev:hover {
    background-position: -86px center;
}

.next {
    left: auto;
    right: 0;
    border-right: 0;
    background-position: -42px center;
}

.next:hover {
    background-position: -139px center;
}
</style>
