<template>
    <CommonHeader :title="$t('拼团')"></CommonHeader>
    <CommonPageHeader></CommonPageHeader>
    <div v-if="!loading" class="seckill-con">
        <div class="inner container">
            <swiper v-if="sloganPcImageList.length > 1" :itemList="sloganPcImageList" :swiperOption="sloganSwiperOption">
                <template v-slot:default="{ item }">
                    <div class="slogan-slide">
                        <el-image class="slogan-img" :src="item" fit="cover" loading="lazy" />
                    </div>
                </template>
            </swiper>
            <img v-else :src="sloganPcImageList[0] || DEFAULT_SLOGAN_IMAGE" alt="" />
        </div>
        <div class="container joining-content">
            <JoiningGoodsList v-model:queryParams="queryParams" v-model:filter="filter"></JoiningGoodsList>
        </div>
    </div>
    <CommonPageFooter></CommonPageFooter>
</template>

<script setup lang="ts">
import JoiningGoodsList from "~/components/common/JoiningGoodsList.vue";
import { computed } from "vue";
import type { QueryParams } from "~/types/joining/joining";
import { getGrouponPromotionImageList } from "@/api/joining/joining";
import { imageFormat } from "~/utils/format";

const DEFAULT_SLOGAN_IMAGE = new URL("../../assets/images/joining/joiningBg.png", import.meta.url).href;
const pcGrouponImageListRaw = ref<string[]>([]);
const loading = ref<boolean>(true);

const sloganPcImageList = computed(() => {
    return pcGrouponImageListRaw.value
        .map((url) => (url ? imageFormat(url) : ""))
        .filter((url) => !!url);
});

const queryParams = computed(() => {
    let query = <QueryParams>{};
    if (filter.page > 1) {
        query.page = filter.page;
    }
    if (filter.shopId > 0) {
        query.shopId = filter.shopId;
    }
    if (filter.keyword != "") {
        query.keyword = filter.keyword;
    }
    return query;
});

const filter: any = reactive({
    page: 1,
    keyword: "",
    shopId: -1
});
const sloganSwiperOption = ref<any>({
    autoplay: {
        delay: 5000,
        disableOnInteraction: false,
        pauseOnMouseEnter: true
    },
    pagination: {
        clickable: true,
        enabled: true
    },
    slidesPerGroup: 1,
    slidesPerView: 1,
    navigation: true,
    effect: "slide",
    loop: true
});

const initGrouponImageList = async () => {
    loading.value = true;
    try {
        const result = await getGrouponPromotionImageList();
        const seen = new Set<string>();
        const uniquePcImages: string[] = [];
        for (const item of result.records) {
            const img = String(item?.pcGrouponImage || "").trim();
            if (!img) continue;
            if (seen.has(img)) continue;
            seen.add(img);
            uniquePcImages.push(img);
        }
        pcGrouponImageListRaw.value = uniquePcImages;
        loading.value = false;
    } catch (error: any) {
        console.log("error", error.message);
        loading.value = false;
    }
};

onMounted(() => {
    initGrouponImageList();
});
</script>

<style scoped lang="less">
.seckill-con {
    //background: rgb(249, 249, 249) url("/assets/images/seckill/bg.png") no-repeat scroll center top;
    .inner {
        position: relative;
        height: 280px;
        margin: 20px auto;
        border-radius: 8px;
        :deep(.swiper) {
            width: 100%;
            height: 280px;
            border-radius: 8px;
            overflow: hidden;
        }
        .slogan-slide {
            width: 100%;
            height: 280px;
        }
        .slogan-img {
            width: 100%;
            height: 280px;
            background-color: #eeeeee;
        }
        img {
            display: block;
            width: 100%;
            height: 280px;
            object-fit: cover;
        }
    }

    /* 轮播分页器与箭头样式：参考 BannerNew.vue */
    .inner :deep(.swiper-pagination-bullet) {
        width: 6px;
        height: 6px;
        background: #ffffffcc;
        opacity: 0.2;
    }
    .inner :deep(.swiper-pagination-bullet-active) {
        background: #fff !important;
        opacity: 1 !important;
    }

    .inner :deep(.swiper-button-prev) {
        box-sizing: border-box;
        left: 0;
        background-color: rgba(0, 0, 0, 0.122);
        width: 40px;
        height: 48px;
        padding-right: 7px;
        line-height: 35px;
        border-radius: 0 24px 24px 0 !important;
        z-index: 2;
        outline: none;
        transition: background-color 0.2s ease;
        &:hover {
            background-color: rgba(0, 0, 0, 0.239);
        }
    }

    .inner :deep(.swiper-button-next) {
        box-sizing: border-box;
        right: 0;
        background-color: rgba(0, 0, 0, 0.122);
        width: 40px;
        height: 48px;
        padding-left: 7px;
        line-height: 35px;
        z-index: 2;
        border-radius: 24px 0 0 24px !important;
        outline: none;
        transition: background-color 0.2s ease;
        &:hover {
            background-color: rgba(0, 0, 0, 0.239);
        }
    }

    .inner :deep(.swiper-button-prev:after),
    .inner :deep(.swiper-button-next:after) {
        font-family: "iconfont-pc";
        content: "\e70d";
        color: #fff;
        font-size: 14px;
    }

    .inner :deep(.swiper-button-prev:after) {
        transform: rotate(180deg);
    }

    .inner :deep(.swiper-button-prev.swiper-button-disabled),
    .inner :deep(.swiper-button-next.swiper-button-disabled) {
        pointer-events: fill;
    }

    padding-bottom: 10px;
}
</style>
