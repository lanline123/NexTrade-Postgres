<template>
    <div v-if="hotCat.length > 0" class="mod_seckill" :style="modStyle">
        <div v-if="showTitle" class="seckill-top">
            <div class="seckill-title">{{ modelValue?.titleName || $t("热门分类") }}</div>
        </div>
        <div class="seckill_list_wrap">
            <div v-if="hotCat.length > 0" class="seckill_list">
                <swiper :key="swiperKey" :itemList="hotCat" :swiperOption="CatProductWrap" style="width: 100%">
                    <template v-slot:default="{ item }">
                        <div class="proditem" style="width: 248px">
                            <NuxtLink
                                :title="item.productName"
                                :to="urlFormat({ path: 'list', seoLink: item.seoLink, id: item.categoryId })"
                                class=""
                                target="_blank"
                            >
                                <div class="seckill-item-image">
                                    <el-image :src="imageFormat(item.categoryPic)" class="pro_pic" loading="lazy" style="transition: opacity 0.2s linear" />
                                </div>
                                <div class="seckill-item__name">
                                    {{ item.categoryName }}
                                </div>
                            </NuxtLink>
                        </div>
                    </template>
                </swiper>
            </div>
            <div v-else class="seckill_list">
                <div v-for="index in 5" class="proditem" style="width: 200px">
                    <div class="seckill-item-image">
                        <HomeSkeleton></HomeSkeleton>
                    </div>
                    <div class="seckill-item__name" style="height: 30px">
                        <HomeSkeleton></HomeSkeleton>
                    </div>
                </div>
            </div>
        </div>
    </div>
</template>
<script lang="ts" setup>
import { computed, onMounted, ref } from "vue";
import { urlFormat } from "@/utils/format";
import { getCategoryHot } from "@/api/product/category";

const isLoading = ref(true);

const props = defineProps({
    modelValue: { type: Object, default: {} }
});

// 是否显示标题（后台「显示标题」开关，0 隐藏，其余显示）
const showTitle = computed(() => Number(props.modelValue?.showTitle ?? 1) !== 0);

// 一屏显示数量（新字段 displayNum / 旧装修 rowNum，限制 1-10）
const displayNum = computed(() => {
    const n = Number(props.modelValue?.displayNum ?? props.modelValue?.rowNum ?? 6);
    return Number.isFinite(n) && n > 0 ? Math.min(10, Math.max(1, n)) : 6;
});

// 内容层背景色（内容设置「背景颜色」）：与容器层（扩展设置背景色）相互独立，叠在容器层之上
const modStyle = computed(() => {
    const bg = props.modelValue?.backgroundColor;
    return bg ? { backgroundColor: bg } : {};
});

const CatProductWrap = computed<any>(() => ({
    autoplay: {
        delay: 5000,
        disableOnInteraction: false, //用户操作swiper之后，是否禁止autoplay
        pauseOnMouseEnter: true //鼠标置于swiper是否时暂停自动切换
    },
    slidesPerGroup: displayNum.value,
    slidesPerView: displayNum.value, // 一屏显示的slide个数  'auto'
    pagination: false,
    navigation: true,
    effect: "slide",
    loop: hotCat.value.length > displayNum.value
}));

// 一屏数量 / 分类数量变化时强制重建 swiper，保证配置即时生效
const swiperKey = computed(() => `${hotCat.value.length}-${displayNum.value}`);
onMounted(() => {
    getHotCatList();
});
const hotCat = ref<any[]>([]);
const getHotCatList = async () => {
    try {
        const result = await getCategoryHot();
        hotCat.value = result || [];
        isLoading.value = false;
    } catch (err) {
        console.error(err);
    }
};
</script>

<style lang="less" scoped>
.mod_seckill {
    display: flex;
    flex-direction: column;
    width: 100%;
    height: 328px;
    border-radius: 8px;
    background-color: rgba(0, 0, 0, 0.02);
    overflow: hidden;
    .seckill-top {
        display: flex;
        justify-content: space-between;
        align-items: center;
        padding: 20px 24px;
        .seckill-title {
            font-size: 24px;
            font-weight: 700;
            color: #1a1a1a;
        }
        .seckill-count-down {
            display: flex;
            align-items: center;
            font-size: 18px;
            color: #1a1a1a;
        }
    }
}

.mod_seckill .seckill_list_wrap {
    position: relative;
    z-index: 1;
    flex: 1;
    height: 100%;
    width: 100%;
}

.mod_seckill .seckill_list {
    position: relative;
    z-index: 1;
    height: 100%;
    overflow: hidden;
    width: 100%;
    display: flex;
}

.mod_seckill .seckill_list :deep(.swiper-slide) {
    width: 248px;
}

.mod_seckill .seckill_list .proditem {
    box-sizing: border-box;
    position: relative;
    z-index: 1;
    width: 216px;
    &:hover {
        .seckill-item-image {
            .pro_pic {
                filter: brightness(105%);
            }
        }
        .seckill-item__name {
            color: var(--main-bg);
        }
    }
}

.mod_seckill .seckill_list .proditem a {
    display: block;
}

.seckill-item-image {
    width: 180px;
    height: 180px;
    margin-left: 14px;
    background: var(--primary-light-9);
    border-radius: 4px;
    overflow: hidden;
}

.mod_seckill .seckill_list .pro_pic {
    transition:
        transform 0.4s ease,
        -webkit-transform 0.4s ease,
        -moz-transform 0.4s ease,
        -o-transform 0.4s ease;
    width: 180px;
    height: 180px;
}

.mod_seckill .seckill_list .seckill-item__name {
    width: 180px;
    margin-top: 12px;
    margin-bottom: 8px;
    padding: 0 14px;
    text-align: center;
    overflow: hidden;
    text-overflow: ellipsis;
    color: #1a1a1a;
    font-size: 16px;
    font-weight: 400;
    line-height: 20px;
    text-transform: capitalize;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    -webkit-box-orient: vertical;
}

:deep(.swiper) {
    box-sizing: border-box;
    padding: 0 12px;
}

:deep(.swiper-wrapper) {
    display: flex;
}

.seckill_list:deep(.swiper-button-prev) {
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

.seckill_list:deep(.swiper-button-next) {
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

.seckill_list:deep(.swiper-button-prev:after),
.seckill_list:deep(.swiper-button-next:after) {
    font-family: "iconfont-pc";
    content: "\e70d";
    color: #fff;
    font-size: 14px;
}

.seckill_list:deep(.swiper-button-prev:after) {
    transform: rotate(180deg);
}

.seckill_list:deep(.swiper-button-prev.swiper-button-disabled),
.seckill_list:deep(.swiper-button-next.swiper-button-disabled) {
    pointer-events: fill;
}
</style>
