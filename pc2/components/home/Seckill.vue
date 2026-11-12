<template>
    <div v-if="seckillList.length > 0" class="mod_seckill">
        <div class="count_down">
            <p class="count_down_tips">{{ $t("限时秒杀") }}</p>
            <!-- <div class="countdown-desc">{{ modelValue.seckillTitle }}</div> -->
            <div class="countdown-desc">{{ $t(modelValue.seckillTitle) }}</div>
            <div class="more-seckill">
                <NuxtLink to="seckill/list/">{{ $t("查看更多") }} <i class="iconfont-pc icon-login-jt"></i></NuxtLink>
            </div>
        </div>
        <div class="seckill_list_wrap">
            <div v-if="seckillList.length > 0" class="seckill_list">
                <swiper :itemList="seckillList" :swiperOption="seckillProductWrap" style="width: 100%">
                    <template v-slot:default="{ item }">
                        <div class="proditem" style="width: 200px">
                            <NuxtLink
                                :title="item.productName"
                                :to="urlFormat({ path: 'product', seoLink: item.seoLink, sn: item.skuSn || item.productSn })"
                                class=""
                                target="_blank"
                            >
                                <div class="seckill-item-image">
                                    <el-image :src="productImage(item)" class="pro_pic" loading="lazy" style="transition: opacity 0.2s linear" :alt="item.productName || ''" />
                                    <img
                                        v-if="styleMap[item.productId]?.borderImage"
                                        class="product-style-border"
                                        :src="styleMap[item.productId].borderImage"
                                        :alt="item.productName || ''"
                                    />
                                </div>
                                <div class="seckill-item__name">{{ item.productName }}</div>
                                <div class="seckill-item__price">
                                    <FormatPrice v-model="item.seckillPrice" class="price price-miaosha" />
                                    <template v-if="item.marketPrice && Number(item.marketPrice) > 0">
                                        <FormatPrice v-model="item.marketPrice" class="price price-origin" />
                                    </template>
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
import { onMounted, ref } from "vue";
import { urlFormat } from "@/utils/format";
import { getHomeSeckill } from "@/api/home/home";
import type { SeckillFilterState } from "~/types/home/seckill";
import { useProductStyle } from "~/hooks";

const isLoading = ref(true);

const props = defineProps({
    modelValue: { type: Object, default: {} }
});

const seckillProductWrap = ref<any>({
    autoplay: {
        delay: 50000000,
        disableOnInteraction: false, //用户操作swiper之后，是否禁止autoplay
        pauseOnMouseEnter: true //鼠标置于swiper是否时暂停自动切换
    },
    slidesPerGroup: 5,
    slidesPerView: 5, // 一屏显示的slide个数  'auto'
    pagination: false,
    navigation: true,
    effect: "slide"
});
onMounted(() => {
    getData();
});
const seckillList = ref<SeckillFilterState[]>([]);
const { styleMap } = useProductStyle(seckillList);
const getData = async () => {
    isLoading.value = true;
    try {
        const result = await getHomeSeckill();
        seckillList.value = result.records;
    } catch (error) {
    } finally {
        isLoading.value = false;
    }
};
</script>

<style lang="less" scoped>
.mod_seckill {
    height: 260px;
    margin-top: 30px;
    display: flex;
}

.more-seckill {
    display: flex;
    justify-content: center;
    margin-top: 10px;

    a {
        background-color: rgba(0, 0, 0, 0.85);
        color: #fff;
        border: 1px solid rgba(0, 0, 0, 0.85);
        border-radius: 50px;
        padding: 4px 12px;
        font-size: 12px;
        display: flex;
        align-items: center;

        i {
            padding-left: 5px;
            font-size: 12px;
        }
    }
}

.mod_seckill .count_down {
    text-decoration: none;
    overflow: hidden;
    position: relative;
    width: 190px;
    height: 100%;
    color: #fff;
    background-image: url("/assets/images/index/qg-bg.png");
    background-size: contain;
    background-position: 50%;
    background-repeat: no-repeat;
}

.mod_seckill .count_down_tips {
    width: 100%;
    text-align: center;
    font-size: 30px;
    font-weight: 700;
    margin-top: 31px;
    height: 45px;
    line-height: 45px;
}

.count_down .countdown-desc {
    margin-top: 90px;
    font-size: 14px;
    text-align: center;
}

.count_down .countdown-desc span {
    font-size: 18px;
    padding-right: 5px;
    vertical-align: middle;
    height: 26px;
    line-height: 26px;
    font-weight: 700;
}

.mod_seckill .count_time {
    display: flex;
    justify-content: center;
    height: 30px;
    text-align: center;
    font-size: 0;
    margin-top: 10px;
}

.mod_seckill .count_time em {
    display: inline-block;
    width: 36px;
    height: 26px;
    line-height: 26px;
    text-align: center;
    background-color: #392800;
    color: #fff;
    font-size: 20px;
    font-family: "微软雅黑";
    font-weight: bold;
}

.mod_seckill .count_time span {
    display: inline-block;
    width: 20px;
    height: 26px;
    line-height: 26px;
    text-align: center;
    font-size: 14px;
    font-weight: bold;
    color: #392800;
    font-style: normal;
}

.mod_seckill .seckill_list_wrap {
    position: relative;
    z-index: 1;
    flex: 1;
    height: 100%;
    background-color: #fff;
    width: 1000px;
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
    width: 200px;
}

.mod_seckill .seckill_list .proditem {
    position: relative;
    z-index: 1;
    width: 200px;
}

.mod_seckill .seckill_list .proditem:after {
    content: "";
    display: block;
    position: absolute;
    top: 50%;
    right: 0;
    width: 1px;
    height: 200px;
    -webkit-transform: translateY(-50%);
    -ms-transform: translateY(-50%);
    transform: translateY(-50%);
    background: -webkit-gradient(linear, left top, left bottom, from(white), color-stop(#eeeeee), to(white));
    background: linear-gradient(180deg, white, #eeeeee, white);
}

.mod_seckill .seckill_list .proditem a {
    display: block;
}

.seckill-item-image {
    position: relative;
    width: 140px;
    height: 140px;
    margin-left: auto;
    margin-right: auto;
    margin-top: 30px;
}

.product-style-border {
    position: absolute;
    left: 0;
    top: 0;
    width: 100%;
    height: 100%;
    z-index: 5;
    pointer-events: none;
    object-fit: fill;
}

.mod_seckill .seckill_list .pro_pic {
    transition:
        transform 0.4s ease,
        -webkit-transform 0.4s ease,
        -moz-transform 0.4s ease,
        -o-transform 0.4s ease;
    width: 140px;
    height: 140px;
}

.mod_seckill .seckill_list .seckill-item__name {
    font-size: 12px;
    font-weight: 400;
    color: #333;
    -webkit-transition: color 0.2s ease;
    transition: color 0.2s ease;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    width: 160px;
    height: 20px;
    line-height: 20px;
    margin-left: auto;
    margin-right: auto;
    margin-top: 13px;
}

.mod_seckill .seckill_list .seckill-item__price {
    display: flex;
    align-items: center;
    border: 1px solid var(--general);
    position: relative;
    -webkit-box-sizing: border-box;
    box-sizing: border-box;
    line-height: 24px;
    overflow: hidden;
    background-color: var(--general);
    width: 160px;
    height: 24px;
    margin-left: auto;
    margin-right: auto;
    margin-top: 7px;
}

.seckill-item__price .price-miaosha {
    width: 92px;
    height: 100%;
    align-items: center;
    justify-content: center;
    color: var(--main-text);
    line-height: 22px;

    :deep(.num) {
        font-weight: 600;
        font-size: 15px;
    }

    :deep(.util) {
        margin-bottom: -2px;
    }
}

.seckill-item__price .price-origin {
    height: 100%;
    width: 66px;
    background: #fff;
    align-items: center;
    justify-content: center;
    color: #999;
    font-size: 12px;
    line-height: 22px;
    -webkit-box-sizing: border-box;
    box-sizing: border-box;
    text-decoration: line-through;
    vertical-align: top;

    b {
        font-size: 12px;
    }
}

.seckill-item__price .price-miaosha:before {
    content: " ";
    width: 0;
    height: 0;
    border-color: transparent white transparent transparent;
    border-style: solid;
    border-width: 22px 8px 0 0;
    position: absolute;
    top: 0;
    left: 84px;
}

:deep(.swiper-wrapper) {
    height: 260px;
    display: flex;
}

.seckill_list:deep(.swiper-button-prev),
.seckill_list:deep(.swiper-button-next) {
    width: 25px;
    height: 35px;
    line-height: 35px;
    background-color: #d9d9d9;
    background-color: rgba(0, 0, 0, 0.15);
    z-index: 2;
    border: none;
    outline: none;
    transition: background-color 0.2s ease;
}

.seckill_list:deep(.swiper-button-prev) {
    left: 0;
    border-top-right-radius: 18px;
    border-bottom-right-radius: 18px;
}

.seckill_list:deep(.swiper-button-next) {
    right: 0;
    border-top-left-radius: 18px;
    border-bottom-left-radius: 18px;
}

.seckill_list:deep(.swiper-button-prev:after),
.seckill_list:deep(.swiper-button-next:after) {
    color: #fff;
    font-size: 14px;
    font-weight: 700;
}

.seckill_list:deep(.swiper-button-prev.swiper-button-disabled),
.seckill_list:deep(.swiper-button-next.swiper-button-disabled) {
    pointer-events: fill;
}
</style>
