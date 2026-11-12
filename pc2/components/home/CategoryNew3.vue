<template>
    <div v-if="modelValue.picList?.length > 0" class="mod_seckill">
         <div class="seckill-top">
            <div class="seckill-title">{{ modelValue.titleText }}</div>
            <div class="seckill-count-down" v-if="modelValue.showMore == 1">
                <NuxtLink :to="urlFormat(modelValue.moreLink)">
                    {{ modelValue.showMoreText }}
                    <SvgIcon class="icon" name="arrow-right" width="16px" height="16px"></SvgIcon>
                </NuxtLink>
            </div>
         </div>
        <div class="seckill_list_wrap">
            <div class="seckill_list">
                <swiper :itemList="modelValue.picList" :swiperOption="CatProductWrap" style="width: 100%">
                    <template v-slot:default="{ item }">
                        <div class="proditem" style="width: 248px">
                            <NuxtLink
                                :title="item.productName"
                                :to="urlFormat(item.picLink)"
                                class=""
                                target="_blank"
                            >
                                <div class="seckill-item-image" :style="`background-color: ${item.imageBackgroundColor};`">
                                    <el-image :src="productImage(item)" class="pro_pic" loading="lazy" style="transition: opacity 0.2s linear" :alt="item.productName || ''" />
                                </div>
                                <div class="seckill-item__name">
                                    {{ item.picTitle }}
                                </div>
                                <div class="desc-wrap">
                                    <SvgIcon class="icon" name="hot" width="16px" height="16px"></SvgIcon>
                                    <div class="desc-txt">{{ item.picDesc }}</div>
                                </div>
                            </NuxtLink>
                        </div>
                    </template>
                </swiper>
            </div>
        </div>
    </div>
</template>
<script lang="ts" setup>
import { onMounted, ref } from "vue";
import { productImage, urlFormat } from "@/utils/format";


const props = defineProps({
    modelValue: { type: Object, default: {} }
});

const CatProductWrap = ref<any>({
    autoplay: {
        delay: 5000,
        disableOnInteraction: false, //用户操作swiper之后，是否禁止autoplay
        pauseOnMouseEnter: true //鼠标置于swiper是否时暂停自动切换
    },
    slidesPerGroup: 6,
    slidesPerView: 6, // 一屏显示的slide个数  'auto'
    pagination: false,
    navigation: true,
    loopFillGroupWithBlank: true,
    effect: "slide",
    loop: true
});
</script>

<style lang="less" scoped>
.mod_seckill {
    display: flex;
    flex-direction: column;
    // 跟随全站 .container 限宽（旧版 1190），勿写死 1264
    width: 100%;
    height: 352px;
    margin-top: 30px;
    border-radius: 8px;
    background-color: rgba(0, 0, 0, .02);
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

            a {
                display: flex;
                align-items: center;
            }

            .icon {
                margin-left: 8px;
            }
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
    // margin-right: auto;
    // margin-top: 30px;
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
    height: 20px;
    margin-top: 12px;
    margin-bottom: 8px;
    padding: 0 14px;
    text-align: center;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
    color: #1a1a1a;
    font-size: 16px;
    font-weight: 400;
    line-height: 20px;
    text-transform: capitalize;
}

.desc-wrap {
    display: flex;
    justify-content: center;
    align-items: center;
    width: 180px;
    padding: 0 14px;
    .icon {

    }
    .desc-txt {
        margin-left: 4px;
        color: #888b94;
        justify-content: center;
        font-size: 16px;
        line-height: 20px;
    }
}

:deep(.swiper) {
    box-sizing: border-box;
    padding: 0 12px;
}

:deep(.swiper-wrapper) {
    // height: 260px;
    display: flex;
}

.seckill_list:deep(.swiper-button-prev) {
    box-sizing: border-box;
    left: 0;
    background-color: rgba(0, 0, 0, .122);
    width: 40px;
    height: 48px;
    padding-right: 7px;
    line-height: 35px;
    border-radius: 0 24px 24px 0 !important;
    z-index: 2;
    outline: none;
    transition: background-color 0.2s ease;
    &:hover {
        background-color: rgba(0, 0, 0, .239);
    }
}

.seckill_list:deep(.swiper-button-next) {
    box-sizing: border-box;
    right: 0;
    background-color: rgba(0, 0, 0, .122);
    width: 40px;
    height: 48px;
    line-height: 35px;
    padding-left: 7px;
    z-index: 2;
    border-radius: 24px 0 0 24px !important;
    outline: none;
    transition: background-color 0.2s ease;
    &:hover {
        background-color: rgba(0, 0, 0, .239);
    }
}

.seckill_list:deep(.swiper-button-prev:after),
.seckill_list:deep(.swiper-button-next:after) {
    font-family: 'iconfont-pc';
    content: '\e70d';
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
