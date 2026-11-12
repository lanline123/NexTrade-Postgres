<template>
    <div v-if="seckillList.length > 0" class="mod_seckill">
        <div class="seckill-top">
            <div class="seckill-title">{{ $t(modelValue.seckillTitle || "限时秒杀") }}</div>
            <div class="seckill-count-down">
                <NuxtLink to="seckill/list/">
                    <CountDown countdown-type="specifyTime" :specify-time="getTime">
                        <template #default="{ days, hours, minutes, seconds }">
                            <div class="countdown-box">
                                <span class="countdown-text">{{ $t("距结束") }}: </span>
                                <template v-if="days && Number(days) > 0">
                                    <span class="countdown-item" :class="{ days: String(days).length > 2 }">{{ days }}</span>
                                    <span class="countdown-separator-text">{{ $t("天") }}</span>
                                </template>
                                <span class="countdown-item">{{ hours }}</span>
                                <span class="countdown-separator">:</span>
                                <span class="countdown-item">{{ minutes }}</span>
                                <span class="countdown-separator">:</span>
                                <span class="countdown-item">{{ seconds }}</span>
                                <SvgIcon class="icon" name="arrow-right" width="16px" height="16px"></SvgIcon>
                            </div>
                        </template>
                    </CountDown>
                </NuxtLink>
            </div>
        </div>
        <div class="seckill_list_wrap">
            <div v-if="seckillList.length > 0" class="seckill_list">
                <swiper :itemList="seckillList" :swiperOption="seckillProductWrap" style="width: 100%">
                    <template v-slot:default="{ item }">
                        <div class="proditem" style="width: 248px">
                            <NuxtLink
                                :title="item.productName"
                                :to="urlFormat({ path: 'product', seoLink: item.seoLink, id: item.productId, sn: item.skuSn || item.productSn })"
                                class=""
                                target="_blank"
                            >
                                <div class="seckill-item-image">
                                    <el-image :src="productImage(item)" class="pro_pic" loading="lazy" style="transition: opacity 0.2s linear" :alt="item.productName || ''" />
                                    <img v-if="styleMap[item.productId]?.borderImage" class="product-style-border" :src="styleMap[item.productId].borderImage || undefined" alt="" />
                                </div>
                                <div class="seckill-item-discount" v-if="item.marketPrice">
                                    {{ $t("优惠{0}%", [getDiscount(Number(item.seckillPrice), Number(item.marketPrice))]) }}
                                </div>
                                <NuxtLink :to="urlFormat({ path: 'product', seoLink: item.seoLink, id: item.productId, sn: item.skuSn || item.productSn })" target="_blank">
                                    <div class="seckill-item__name">
                                        {{ item.productName }}
                                    </div>
                                </NuxtLink>
                            </NuxtLink>
                            <div class="seckill-item__price">
                                <FormatPrice v-model="item.seckillPrice" class="price price-miaosha" />
                                <!-- <div class="buy">
                                        <NuxtLink :to="urlFormat({ path: 'product', seoLink: item.seoLink, id: item.productId, sn: item.skuSn || item.productSn })" target="_blank">
                                            <div class="view_detail">
                                                <SvgIcon name="add" width="28px" height="28px"></SvgIcon>
                                            </div>
                                        </NuxtLink>
                                    </div> -->
                                <div class="buy">
                                    <div v-if="item.seckkillData.seckillItem && item.seckkillData.seckillItem.length > 1">
                                        <NuxtLink @click.stop :to="urlFormat({ path: 'product', seoLink: item.seoLink, id: item.productId, sn: item.productSn })" target="_blank">
                                            <div class="view_detail">
                                                <SvgIcon name="add" width="32px" height="32px"></SvgIcon>
                                            </div>
                                        </NuxtLink>
                                    </div>
                                    <div v-else>
                                        <ProductBuy
                                            @click.stop
                                            :isQuick="false"
                                            :id="item.productId"
                                            :disabled="(item?.isSeckill == 0 && item?.productStock == 0) || (item?.isSeckill == 1 && item?.seckillStock == 0)"
                                            :number="1"
                                            @callback="$emit('callback')"
                                        >
                                            <div class="view_detail">
                                                <SvgIcon name="add" width="32px" height="32px"></SvgIcon>
                                            </div>
                                        </ProductBuy>
                                    </div>
                                </div>
                            </div>
                            <template v-if="item.marketPrice && Number(item.marketPrice) > 0">
                                <FormatPrice v-model="item.marketPrice" class="price price-origin" />
                            </template>
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
import { productImage, urlFormat } from "@/utils/format";
import { getHomeSeckill } from "@/api/home/home";
import type { SeckillFilterState } from "~/types/home/seckill";
import CountDown from "~/components/common/CountDown.vue";
import { useProductStyle } from "~/hooks";

const isLoading = ref(true);

const props = defineProps({
    modelValue: { type: Object, default: {} }
});
const getDiscount = (currentPrice: number, originalPrice: number) => {
    if (originalPrice === 0 || currentPrice >= originalPrice) {
        return 0;
    }

    const discountPercentage = ((originalPrice - currentPrice) / originalPrice) * 100;

    return Math.floor(discountPercentage * 10) / 10;
};
const seckillProductWrap = ref<any>({
    autoplay: {
        delay: 5000,
        disableOnInteraction: false, //用户操作swiper之后，是否禁止autoplay
        pauseOnMouseEnter: true //鼠标置于swiper是否时暂停自动切换
    },
    slidesPerGroup: 5,
    slidesPerView: 5, // 一屏显示的slide个数  'auto'
    loopFillGroupWithBlank: true,
    pagination: false,
    navigation: true,
    effect: "slide",
    loop: true
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

const getTime = computed(() => {
    return seckillList.value[0] ? new Date(seckillList.value[0]?.seckkillData?.seckillEndTime).getTime() : 0;
});
</script>

<style lang="less" scoped>
.mod_seckill {
    display: flex;
    flex-direction: column;
    width: 100%;
    height: 428px;
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
            text-transform: capitalize;
        }
        .seckill-count-down {
            display: flex;
            align-items: center;
            font-size: 18px;
            color: #1a1a1a;
        }
    }
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

.countdown-box {
    &:hover {
        .countdown-text {
            color: var(--main-bg);
        }
        .countdown-item {
            background-color: var(--primary-dark-2);
        }
    }
    .countdown-separator-text {
        font-size: 18px;
    }
    .countdown-text {
        font-size: 18px;
        font-weight: 400;
        line-height: 24px;
        color: #1a1a1a;
        text-transform: capitalize;
    }
    .countdown-item {
        display: inline-block;
        padding: 2px 0;
        width: 32px;
        text-align: center;
        background-color: #1a1a1a;
        color: #fff;
        border-radius: 4px;

        &.days {
            width: auto;
            min-width: 32px;
            padding: 0 5px;
        }
    }

    .icon-youjiantou {
        color: #1a1a1a;
        font-size: 14px;
        padding-left: 4px;
    }

    span {
        margin: 0 1px;
    }
    .icon {
        margin-left: 8px;
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
    width: 248px;
    .seckill-item-discount {
        margin: 12px 0 8px 14px;
        width: fit-content;
        height: 20px;
        line-height: 20px;
        color: #fff;
        text-align: center;
        border-radius: 2px;
        background-color: var(--main-bg);
        padding: 2px 6px;
        font-size: 14px;
    }
    &:hover {
        .seckill-item-image {
            .pro_pic {
                filter: brightness(105%);
            }
        }
    }
}

.mod_seckill .seckill_list .proditem a {
    display: block;
}

.seckill-item-image {
    position: relative;
    width: 220px;
    height: 220px;
    margin-left: 14px;
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
    width: 220px;
    height: 220px;
}

.mod_seckill .seckill_list .seckill-item__name {
    height: 20px;
    margin-top: 12px;
    margin-bottom: 8px;
    padding: 0 14px;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
    color: #1a1a1a;
    font-size: 16px;
    font-weight: 400;
    line-height: 18px;
    cursor: pointer;
    &:hover {
        color: var(--main-bg);
    }
}
.mod_seckill .seckill_list .seckill-item__price {
    display: flex;
    align-items: center;
    padding: 0 14px;
    .buy {
        margin-left: auto;
        .view_detail {
            display: flex;
            justify-content: center;
            align-items: center;
            width: 32px;
            height: 32px;
            border-radius: 50%;
            background: var(--primary-light-9);
            color: var(--main-bg);
            &:hover {
                color: var(--primary-dark-2);
            }
            &.disabled {
                opacity: 0.5;
                cursor: not-allowed;
            }
        }
    }
}

.seckill-item__price .price-miaosha {
    align-items: baseline;
    justify-content: center;
    margin-right: 8px;
    color: #1a1a1a;
    line-height: 22px;

    :deep(.price-row) {
        align-items: baseline !important;
    }

    :deep(.num) {
        font-size: 24px;
        color: #1a1a1a;
        font-weight: 600;
    }

    :deep(.util) {
        color: #1a1a1a;
        font-size: 16px;
    }
    :deep(.decimal) {
        color: #1a1a1a;
        font-size: 16px;
    }
}

.seckill-item__price .price-origin {
    align-items: baseline;
    justify-content: center;
    height: 19px;
    color: #888b94;
    text-decoration: line-through;
    :deep(.price-row) {
        line-height: 22px;
        font-weight: 500;
    }
    :deep(.num) {
        font-size: 16px;
        color: #888b94;
    }

    :deep(.util) {
        color: #1a1a1a;
        color: #888b94;
    }
}

.price-origin{
    padding: 0 14px;
    box-sizing: border-box;
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
