<template>
    <div class="simple-ads">
        <swiper v-model:swiperOption="swiperOption" :itemList="value.picList">
            <template v-slot:default="{ item, index }">
                <li>
                    <NuxtLink
                        :to="urlFormat(item.picLink)"
                        target="_blank"
                        :style="backgroundImageStyle(imageFormat(item.picUrl))"
                        role="img"
                        :aria-label="getAltText(item)"
                        :title="getAltText(item)"
                    />
                </li>
            </template>
        </swiper>
    </div>
</template>
<script lang="ts" setup>
import { ref } from "vue";
import { imageFormat, urlFormat } from "@/utils/format";
import { useCommonStore } from "~/store/common";

const props = defineProps({
    value: {
        type: Object,
        default: {}
    }
});

const commonStore: any = useCommonStore();
const siteName = computed(() => commonStore.shopTitle || commonStore.shopName || "");

const getAltText = (item: any) => {
    return (
        item?.picTitle?.toString().trim() ||
        item?.picDesc?.toString().trim() ||
        item?.picName?.toString().trim() ||
        siteName.value
    );
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
    navigation: false,
    effect: "slide",
    loop: true
});

const backgroundImageStyle = (url: string) => {
    return {
        backgroundImage: `url('${url}')`
    };
};
</script>
<style lang="less" scoped>
.simple-ads {
    position: absolute;
    top: 0;
    z-index: 1;
    overflow: hidden;
}

.simple-ads,
.simple-ads li,
.simple-ads li a {
    width: 100%;
    height: 100%;
    display: block;
}

.simple-ads li a {
    background-repeat: no-repeat;
    background-position: center center;
    background-size: cover;
}

:deep(.swiper) {
    height: 100%;
}

:deep(.swiper-pagination) {
    text-align: center;
    bottom: 24px !important;
}

:deep(.swiper-pagination-bullet) {
    position: relative;
    display: inline-block;
    transition: background 0.2s ease;
    margin: 0 6px;
    height: 8px;
    width: 8px;
    border-radius: 8px;
    background-color: rgba(0, 0, 0, 0.2);
}

:deep(.swiper-pagination-bullet-active) {
    width: 12px;
    background-color: rgb(255, 255, 255);
    box-shadow: 1px 2px 5px rgba(0, 0, 0, 0.3);
}
</style>
