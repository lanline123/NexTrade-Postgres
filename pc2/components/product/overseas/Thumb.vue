<template>
    <div class="flex image_wid">
        <div class="slider">
            <div class="slider-content">
                <div class="slider-wrapper flex">
                    <template v-for="(item, index) in list" :key="index">
                        <div class="slider-item">
                            <div class="item" :class="isThumbActive(index) ? 'cur' : ''">
                                <img :src="item.videoCover ? imageFormat(item.videoCover) : productImage(item)" :alt="productName || ''" @mouseover="handleMouseOver(index)" />
                                <template v-if="item.videoUrl">
                                    <div class="video-icon"></div>
                                </template>
                            </div>
                        </div>
                    </template>
                </div>
            </div>
        </div>
        <div class="gallery">
            <PicShow @play="showVideo = true" v-show="!showVideo" :images="activeImage" :showEnquiry="showEnquiry" :productName="productName" />

            <template v-if="!(productStock && productStock > 0)">
                <div class="outsale">
                    {{ $t("已售罄") }}
                </div>
            </template>

            <template v-if="videoData.videoUrl">
                <productVideo
                    v-show="showVideo"
                    :showVideo="showVideo"
                    :src="videoData.videoUrl"
                    :type="`video/${videoData.format}`"
                    class="video-box"
                    @ended="showVideo = false"
                ></productVideo>
            </template>
        </div>
        <span class="iconfont-pc collect ico-color collection" @click="addCollect(isCollect)">
            <SvgIcon v-if="!isCollect" name="loveSolid" width="24px" height="24px" class="svgiconGround"></SvgIcon>
            <SvgIcon v-else name="love" width="26px" height="26px" class="svgiconGround"></SvgIcon>
        </span>
    </div>
</template>

<script setup lang="ts">
import PicShow from "./PicShow.vue";
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
    productName: {
        type: String,
        default: ""
    },
    isCollect: {
        type: Boolean
    },
    productStock: {
        type: Number,
        default: 1
    },
    showEnquiry: {
        type: Boolean
    }
});
const emits = defineEmits(["addCollectEmpty"]);

const videoData = computed(() => {
    return props.videoList[0] ? props.videoList[0] : ({} as VideoList);
});
const addCollect = (isCollect: boolean) => {
    emits("addCollectEmpty", isCollect);
};

const list = computed(() => {
    return videoData.value.videoUrl
        ? [
              {
                  ...videoData.value,
                  picUrl: pickPic(props.picList[0]),
                  // 视频项补一张大图，播完回封面时 PicShow 才能正常渲染
                  picLarge: videoData.value.videoCover || props.picList[0]?.picLarge || props.picList[0]?.picUrl || ""
              },
              ...props.picList
          ]
        : props.picList;
});

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
    showVideo.value = list.value[imgIndex.value].videoUrl ? true : false;
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
</script>

<style lang="less" scoped>
.gallery {
    width: 624px;
    position: relative;

    .outsale {
        position: absolute;
        top: 50%;
        left: 50%;
        width: 50%;
        height: 50%;
        transform: translate(-50%, -50%);
        background: rgba(0, 0, 0, 0.4);
        font-size: 28px;
        color: #ffffff;
        letter-spacing: 0;
        border-radius: 100%;
        display: flex;
        align-items: center;
        justify-content: center;
    }

    .video-box {
        width: 624px;
        height: 624px;
    }

    img {
        width: 624px;
        height: 358px;
        font-size: 5px;
    }
}
// .slider {
//     width: 360px;
//     position: relative;
//     display: flex;
//     justify-content: space-between;

//     .slider-content {
//         overflow: hidden;
//         width: 310px;
//     }
// }

.slider-wrapper {
    padding: 2px 0px;
    display: flex;
    flex-shrink: 0;
    flex-direction: column;
    gap: 16px;
    width: 112px;
    height: 624px;
    overflow-y: auto;
    scrollbar-width: none;
    &::-webkit-scrollbar{
        display: none;
    }
    // align-items: center;
    // transition: transform 0.5s;
    // overflow: hidden;
}

.slider-item {
    cursor: pointer;
    box-sizing: border-box;
    width: 100%;
    /* 假设每张图片宽度为400px */
    .item {
        display: flex;
        justify-content: center;
        align-items: center;
        box-sizing: border-box;
        width: 108px;
        border-radius: 4px;
        border: transparent solid 2px;
        overflow: hidden;
        img {
            width: 104px;
            height: 104px;
            object-fit: contain;
            object-position: center;
        }

        &.cur {
            // border-color: var(--general);
            border: #1a1a1a solid 2px;
        }
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
.image_wid {
    display: flex;
    gap: 16px;
    margin-bottom: 16px;
    user-select: none;
    position: relative;
}
.collection {
    width: 48px;
    height: 48px;
    position: absolute;
    // z-index: 66;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    border: 1px solid rgba(0, 0, 0, 0.04);
    border-radius: 6px;
    background: rgb(255, 255, 255);
    right: 12px;
    top: 12px;
    font-size: 24px;
}
.svgiconGround {
    path {
        fill: rgb(204, 12, 28);
        stroke: rgb(204, 12, 28);
    }
}
</style>
