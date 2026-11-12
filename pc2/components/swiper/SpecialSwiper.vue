<template>
    <div class="floor-slider" @mouseover="stopAutoPlay" @mouseout="startAutoPlay">
        <ul class="slides">
            <li v-for="(slide, index) in slides" :key="index" :style="getSlideStyle(index)" class="slide">
                <NuxtLink target="_blank" class="a-cat" title="" :to="urlFormat(slide.picLink)">
                    <p class="caption" :style="'color: ' + activeColor">{{ slide.picTitle }}</p>
                    <p class="sub_tit">{{ $t(slide.picDesc) }}</p>
                    <img :src="imageFormat(slide.picUrl)" :alt="slide.picTitle || ''" height="130" width="130" />
                </NuxtLink>
                <div class="color_mask_4 color_mask" style="opacity: 0"></div>
            </li>
        </ul>
        <div class="turn_show">
            <div class="prev_btn" @click="navigate(-1)">&lt;</div>
            <div class="show_num">
                <span>{{ currentSlide + 1 }}</span> / <em>{{ slides.length }}</em>
            </div>
            <div class="next_btn" @click="navigate(1)">&gt;</div>
        </div>
    </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted, reactive } from "vue";
import { urlFormat } from "~/utils/format";
const props = defineProps({
    modelValue: { type: Array, default: [] },
    activeColor: { type: String }
});
const activeColor = ref(props.activeColor);

interface ModulePicListType {
    picUrl?: string;
    picThumb?: string;
    picLink?: {
        link?: string;
        title?: string;
        id?: number;
        sn?: string;
        path?: string;
    };
    picTitle?: string;
    picDesc?: string;
}
type SlideStyle = {
    width: string;
    height: string;
    left: string;
    top: string;
    opacity: number;
    zIndex: number;
};

const slides = ref(<ModulePicListType[]>props.modelValue);
const currentSlide = ref(0);
const slideStyles = reactive<Record<number, SlideStyle>>({});
let autoPlayInterval: number | undefined;

const defaultStyles: SlideStyle = {
    width: "195px",
    height: "225px",
    left: "0px",
    top: "0px",
    opacity: 1,
    zIndex: 100
};

const updateSlideStyles = () => {
    slides.value.forEach((_, index) => {
        if (index === currentSlide.value) {
            slideStyles[index] = { ...defaultStyles };
        } else if (index === currentSlide.value - 1 || (currentSlide.value === 0 && index === slides.value.length - 1)) {
            slideStyles[index] = {
                width: "155px",
                height: "225px",
                left: "20px",
                top: "-18px",
                opacity: 0.5,
                zIndex: 80
            };
        } else if (index === currentSlide.value + 1 || (currentSlide.value === slides.value.length - 1 && index === 0)) {
            slideStyles[index] = {
                width: "175px",
                height: "225px",
                left: "10px",
                top: "-10px",
                opacity: 0.8,
                zIndex: 90
            };
        } else {
            slideStyles[index] = {
                ...defaultStyles,
                opacity: 0,
                zIndex: 0
            };
        }
    });
};

const navigate = (direction: number) => {
    const numberOfSlides = slides.value.length;
    currentSlide.value = (currentSlide.value + direction + numberOfSlides) % numberOfSlides;
    updateSlideStyles();
};

const getSlideStyle = (index: number): SlideStyle => {
    return slideStyles[index] || defaultStyles;
};

const startAutoPlay = () => {
    stopAutoPlay();
    autoPlayInterval = window.setInterval(() => navigate(1), 3000);
};

const stopAutoPlay = () => {
    if (autoPlayInterval) {
        window.clearInterval(autoPlayInterval);
    }
};

onMounted(() => {
    updateSlideStyles();
    startAutoPlay();
});

onUnmounted(() => {
    stopAutoPlay();
});

// Exporting currentSlide to use it in the template for reactivity
</script>

<style scoped>
.floor-slider {
    position: relative;
    margin-top: 35px;

    ul {
        width: 195px;
        height: 225px;
        margin: 0 auto;
        position: relative;
    }

    li {
        position: absolute;
        width: 195px;
        height: 225px;
        margin: 0 auto;
        -moz-border-radius: 4px;
        border-radius: 4px;
        background-color: #fff;
        transition: all 0.5s;
    }

    li a.a-cat {
        display: block;
        height: 205px;
        padding-top: 20px;
    }

    li a.a-cat:hover {
        text-decoration: none;
    }

    .caption {
        color: #f78b4b;
    }

    .caption {
        height: 20px;
        overflow: hidden;
        font-size: 16px;
        line-height: 20px;
    }

    .sub_tit {
        height: 30px;
        overflow: hidden;
        font-size: 14px;
        color: #666;
        line-height: 30px;
    }

    li img {
        width: 130px;
        height: 130px;
        margin-top: 5px;
    }

    .color_mask {
        position: absolute;
        top: 0;
        left: 0;
        z-index: -1;
        width: 100%;
        height: 225px;
        -moz-border-radius: 4px;
        border-radius: 4px;
    }

    .turn_show {
        position: absolute;
        top: 235px;
        left: 67px;
        height: 30px;
        line-height: 30px;
        color: #f8f6f7;
        display: flex;
        align-items: center;
    }

    .prev_btn,
    .next_btn {
        text-align: center;
        font-size: 14px;
        font-weight: bold;
        cursor: pointer;
    }

    .show_num {
        padding: 0 20px;
        font-size: 14px;
    }
}
</style>
