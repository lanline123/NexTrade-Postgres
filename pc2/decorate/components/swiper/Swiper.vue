<template>
    <!-- 使用 key 强制在数据量变化时重新初始化 Swiper -->
    <Swiper
        v-if="props.itemList && props.itemList.length > 0"
        :key="props.itemList.length"
        :modules="[Autoplay, Pagination, Navigation, EffectFade]"
        v-bind="computedSwiperOption"
    >
        <swiper-slide v-for="(item, index) in props.itemList" :key="index">
            <slot :item="item" :index="index"></slot>
        </swiper-slide>
    </Swiper>
</template>
<script setup lang="ts">
import type { PropType } from "vue";
import { computed, ref } from "vue";

import { Swiper, SwiperSlide } from "swiper/vue";
import { Autoplay, EffectFade, Navigation, Pagination } from "swiper/modules";
import "swiper/css";
import "swiper/css/navigation";
import "swiper/css/pagination";
import "swiper/css/effect-fade";

const props = defineProps({
    swiperOption: { type: Object },
    itemList: { type: Array as PropType<any[]>, default: () => [] },
    activeBulletStyle: { type: String }
});

const activeBulletStyle = ref<string | undefined>(props.activeBulletStyle);

const defaultSwiperOption = ref<Record<string, unknown>>({
    autoplay: {
        delay: 3000,
        disableOnInteraction: false,
        pauseOnMouseEnter: true
    },
    pagination: {
        clickable: true,
        enabled: false,
        renderBullet: function (index: number, className: string): string {
            return `<span class="${className}" style="` + activeBulletStyle.value + `"></span>`;
        }
    },
    slidesPerView: 1,
    slidesPerGroup: 1,
    navigation: false,
    effect: "slide",
    loop: false
});

const toNumber = (val: unknown, defaultVal: number): number => {
    if (typeof val === "number") return val;
    if (typeof val === "string") {
        const num = Number(val);
        return isNaN(num) ? defaultVal : num;
    }
    return defaultVal;
};

const computedSwiperOption = computed<Record<string, unknown>>(() => {
    const merged: Record<string, unknown> = { ...defaultSwiperOption.value, ...props.swiperOption };
    const itemCount = props.itemList?.length || 0;

    const rawSlidesPerView = merged.slidesPerView;
    const rawSlidesPerGroup = merged.slidesPerGroup;

    const slidesPerGroup = toNumber(rawSlidesPerGroup, 1);
    merged.slidesPerGroup = slidesPerGroup;

    const slidesPerView =
        rawSlidesPerView === "auto"
            ? "auto"
            : typeof rawSlidesPerView === "number"
              ? rawSlidesPerView
              : typeof rawSlidesPerView === "string" && rawSlidesPerView.trim() !== "" && !isNaN(Number(rawSlidesPerView))
                ? Number(rawSlidesPerView)
                : 1;

    if (slidesPerView !== "auto") {
        merged.slidesPerView = slidesPerView;
    }
    if (merged.loopAdditionalSlides !== undefined) {
        merged.loopAdditionalSlides = toNumber(merged.loopAdditionalSlides, 0);
    }

    const minSlidesForLoop = slidesPerView === "auto" ? 2 : Math.max(slidesPerView + slidesPerGroup, slidesPerView * 2);

    if (merged.loop && itemCount < minSlidesForLoop) {
        merged.loop = false;
    }

    return merged;
});
</script>
