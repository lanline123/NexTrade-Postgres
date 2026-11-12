<template>
    <div class="mod_columns_wrap">
        <div class="mod_columns_floor">
            <div :style="leftConStyle" class="floor_tit acea-row row-between-wrapper">
                <h4 class="acea-row row-middle">
                    <NuxtLink :title="module.title" :to="urlFormat({ path: 'list', seoLink: module.seoLink, id: module.categoryId })" target="_blank">{{
                        module.title
                    }}</NuxtLink>
                    <span class="iconfont-pc icon-yuanjiaojiantouyou f10 ml10"></span>
                </h4>
                <div class="keywords">
                    <ul class="acea-row row-middle">
                        <li v-for="(item, index) in module.childCategoryList?.slice(0, 6)" :key="index">
                            <NuxtLink :to="urlFormat({ path: 'list', seoLink: item.seoLink, id: item.categoryId })" :title="item.categoryName" target="_blank">{{
                                item.categoryName
                            }}</NuxtLink>
                            |
                        </li>
                    </ul>
                </div>
            </div>
            <div class="acea-row">
                <div class="left_slider">
                    <swiper :activeBulletStyle="activeBulletStyle" :itemList="module.picList" :swiperOption="homeCatWrap">
                        <template v-slot:default="{ item }">
                            <li>
                                <NuxtLink :title="item.picTitle" :to="urlFormat(item.picLink)" target="_blank">
                                    <p :style="textColorStyle" class="caption">{{ item.picTitle }}</p>
                                    <p class="sub_tit">{{ $t(item.picDesc) }}</p>
                                    <span class="pic-img-box">
                                        <img :alt="item.picTitle" :src="imageFormat(item.picUrl)" />
                                    </span>
                                </NuxtLink>
                            </li>
                        </template>
                    </swiper>
                </div>
                <div class="pro_con_warp">
                    <div class="pro_con acea-row">
                        <div v-for="(item, index) in module.picList2" :key="index" class="pro_wrap">
                            <NuxtLink v-if="index < 4" :title="item.picTitle" :to="urlFormat(item.picLink)" target="_blank">
                                <p class="pro_tit">{{ item.picTitle }}</p>
                                <p :style="textColorStyle" class="pro_sub_tit">{{ $t(item.picDesc) }}</p>
                                <span class="pic-img-box pic-img-box--small">
                                    <img :alt="item.picTitle" :src="imageFormat(item.picUrl)" />
                                </span>
                            </NuxtLink>
                        </div>
                    </div>
                </div>
            </div>
        </div>
    </div>
</template>
<script lang="ts" setup>
import { ref } from "vue";
import { urlFormat } from "@/utils/format";
import "swiper/css/pagination";
const homeCatWrap = ref<any>({
    autoplay: {
        delay: 5000,
        disableOnInteraction: false, //用户操作swiper之后，是否禁止autoplay
        pauseOnMouseEnter: true //鼠标置于swiper是否时暂停自动切换
    },
    effect: "fade"
});

const props = defineProps({
    modelValue: { type: Object }
});

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

interface ModuleLinkListType {
    link?: {
        link?: string;
        title?: string;
    };
    linkTitle?: string;
}

interface PcCatProductType {
    categoryId?: number;
    seoLink?: string;
    childCategoryList?: {
        categoryId: number;
        categoryName: string;
        seoLink?: string;
    }[];
    productList?: {
        picThumb?: string;
        productName?: string;
        productPrice?: string;
        productSn?: string;
        productId?: number;
    }[];
    shortTitle?: string;
    title?: string;
    enTitle?: string;
    color?: string;
    color2?: string;
    picList?: ModulePicListType[];
    picList2?: ModulePicListType[];
    picList3?: ModulePicListType[];
    linkList?: ModuleLinkListType[];
    productIds?: number[];
}

const module = ref(<PcCatProductType>props.modelValue);

const leftConStyle = {
    background: "linear-gradient(30deg, " + module.value.color + " 0%, " + module.value.color2 + " 100%)"
};
const textColorStyle = {
    color: module.value.color
};
const activeBulletStyle = `color: ` + module.value.color + `;background-color: ` + module.value.color;
</script>
<style lang="less" scoped>
.acea-row {
    display: flex;
    flex-wrap: wrap;
}

.acea-row.row-middle {
    align-items: center;
}

.acea-row.row-right {
    justify-content: flex-end;
}

.acea-row.row-center-wrapper {
    align-items: center;
    justify-content: center;
}

.acea-row.row-between-wrapper {
    align-items: center;
    justify-content: space-between;
}

.f10 {
    font-size: 10px;
}

.ml10 {
    margin-left: 10px;
}

// 两列
.mod_columns_wrap {
    overflow: hidden;
    margin-top: 20px;
}

.mod_columns_wrap .mod_columns_floor {
    width: 608px;
    height: 430px;
    background-color: #fff;
}

.mod_columns_wrap .floor_tit {
    padding: 12px 10px 12px 18px;
    height: 25px;
    background: rgba(0, 0, 0, 0) linear-gradient(to right, #0bcd96 0, #05d7a8 100%) repeat scroll 0 0;
    color: #fff;
    line-height: 25px;
}

.mod_columns_wrap .floor_tit h4 {
    font-weight: 400;
    font-size: 20px;
}

.mod_columns_wrap .floor_tit .keywords {
    overflow: hidden;
    height: 24px;
    max-width: 430px;
    text-align: right;
    line-height: 24px;
}

.mod_columns_wrap .floor_tit a {
    color: #fff;
}

.mod_columns_wrap .floor_tit a:hover {
    text-decoration: none;
}

.mod_columns_wrap .floor_tit .keywords ul {
    margin-right: -5px;
}

.mod_columns_wrap .floor_tit .keywords li {
    position: relative;
}

.mod_columns_wrap .floor_tit .keywords a {
    margin: 0 8px;
}

.mod_columns_wrap .floor_tit .keywords a:hover {
    color: #fff;
    text-decoration: underline;
}

.mod_columns_wrap .floor_tit a {
    color: #fff;
}

.mod_columns_wrap .left_slider {
    position: relative;
    overflow: hidden;
    width: 218px;
    height: 381px;
}

.mod_columns_wrap .pro_con_warp {
    position: relative;
}

.mod_columns_wrap .pro_con {
    overflow: hidden;
    width: 372px;
    height: 381px;
}

.mod_columns_wrap .left_slider li {
    position: relative;
}

.mod_columns_wrap .left_slider li a {
    display: block;
    padding-top: 25px;
    width: 218px;
    height: 356px;
}

.mod_columns_wrap .left_slider li a:hover {
    text-decoration: none;
}

.mod_columns_wrap .left_slider .caption {
    position: relative;
    z-index: 105;
    margin-left: 18px;
    font-size: 20px;
    height: 24px;
    line-height: 24px;
    overflow: hidden;
}

.mod_columns_wrap .left_slider .sub_tit {
    position: relative;
    z-index: 105;
    margin-left: 18px;
    color: #999;
    font-size: 14px;
    line-height: 30px;
}

.mod_columns_wrap .left_slider li a .pic-img-box {
    position: absolute;
    top: 0;
    left: 0;
    z-index: 102;
    width: 228px;
    height: 381px;
    display: block;
}

.mod_columns_wrap .left_slider li a .pic-img-box > img:first-child {
    position: absolute;
    top: 0;
    left: 0;
    width: 228px;
    height: 381px;
    transition:
        transform 0.4s ease 0s,
        -webkit-transform 0.4s ease 0s,
        -moz-transform 0.4s ease 0s,
        -o-transform 0.4s ease 0s;
}

.mod_columns_wrap .left_slider li a:hover .pic-img-box > img:first-child {
    transform: translate3d(-10px, 0, 0);
}

.mod_columns_wrap .left_slider :deep(.swiper-pagination) {
    position: absolute;
    top: 87px;
    left: 15px;
    z-index: 105;
    height: 18px;
    text-align: left;
}

.mod_columns_wrap .left_slider :deep(.swiper-pagination .swiper-pagination-bullet) {
    overflow: hidden;
    width: 22px;
    height: 4px;
    border-radius: 0;
}

.mod_columns_wrap .left_slider :deep(.swiper-pagination .swiper-pagination-bullet.swiper-pagination-bullet-active) {
    background-color: #0bcd96;
}

.mod_columns_wrap .mod_columns_floor {
    width: 590px;
    height: 430px;
    background-color: #fff;
}

.mod_columns_wrap .pro_wrap {
    box-sizing: border-box;
    position: relative;
    width: 185px;
    height: 190px;
    border-bottom: 1px solid #f5f5f5;
    border-left: 1px solid #f5f5f5;
}

.mod_columns_wrap .pro_wrap a {
    display: block;
    padding-top: 20px;
    height: 170px;
}

.mod_columns_wrap .pro_wrap a:hover {
    text-decoration: none;
}

.mod_columns_wrap .pro_wrap .pro_tit {
    overflow: hidden;
    margin-left: 20px;
    height: 24px;
    max-width: 100px;
    color: #333;
    font-size: 14px;
    line-height: 24px;
}

.mod_columns_wrap .pro_wrap .pro_sub_tit {
    position: relative;
    z-index: 105;
    overflow: hidden;
    margin-left: 20px;
    height: 24px;
    max-width: 100px;
    font-size: 14px;
    line-height: 24px;
}

.mod_columns_wrap .pro_wrap .pic-img-box--small {
    position: absolute;
    right: 10px;
    bottom: 10px;
    z-index: 101;
    width: 105px;
    height: 105px;
    display: block;
}

.mod_columns_wrap .pro_wrap .pic-img-box--small > img:first-child {
    position: absolute;
    left: 0;
    top: 0;
    width: 105px;
    height: 105px;
    transition:
        transform 0.4s ease 0s,
        -webkit-transform 0.4s ease 0s,
        -moz-transform 0.4s ease 0s,
        -o-transform 0.4s ease 0s;
}

.mod_columns_wrap .pro_wrap a:hover .pic-img-box--small > img:first-child {
    transform: translate3d(-5px, 0, 0);
}

.index_floor_1 .floor_tit {
    background: #0bcd96;
    background: -moz-linear-gradient(left, #0bcd96 0%, #05d7a8 100%);
    background: -webkit-linear-gradient(left, #0bcd96 0%, #05d7a8 100%);
    background: -webkit-gradient(linear, left top, right top, from(#0bcd96), to(#05d7a8));
    background: -o-linear-gradient(left, #0bcd96 0%, #05d7a8 100%);
    background: linear-gradient(to right, #0bcd96 0%, #05d7a8 100%);
}

.index_floor_1 .left_slider .caption {
    color: #0bcd96;
}

.index_floor_1 .pro_wrap .pro_sub_tit {
    color: #0bcd96;
}

.index_floor_2 .floor_tit {
    background: #96374e;
    background: -moz-linear-gradient(left, #96374e 0%, #963759 100%);
    background: -webkit-linear-gradient(left, #96374e 0%, #963759 100%);
    background: -webkit-gradient(linear, left top, right top, from(#96374e), to(#963759));
    background: -o-linear-gradient(left, #96374e 0%, #963759 100%);
    background: linear-gradient(to right, #96374e 0%, #963759 100%);
}

.index_floor_2 .left_slider .caption {
    color: #96374e;
}

.index_floor_2 .pro_wrap .pro_sub_tit {
    color: #96374e;
}

.index_floor_2 .left_slider :deep(.swiper-pagination .swiper-pagination-bullet.swiper-pagination-bullet-active) {
    background-color: #96374e;
}
</style>
