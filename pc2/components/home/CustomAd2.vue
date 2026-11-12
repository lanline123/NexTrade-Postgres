<template>
    <div class="mod_global_imported">
        <div class="acea-row">
            <div class="global_food acea-row" style="background: #b96131; background: linear-gradient(#b96131 0%, #6a452b 100%)">
                <div class="left_banner">
                    <NuxtLink v-if="mainPicture" :title="mainPicture?.picTitle ?? ''" :to="urlFormat(mainPicture?.picLink?.data)" target="_blank">
                        <el-image
                            class="img"
                            :alt="mainPicture?.picTitle ?? ''"
                            :src="imageFormat(mainPicture.picUrl)"
                            style="width: 316px; height: 450px"
                        />
                    </NuxtLink>
                    <div class="banner_link">
                        <ul class="acea-row">
                            <li v-for="(item, index) in module.linkList" :key="index">
                                <NuxtLink v-if="index < 6" :title="$t(item.linkTitle ?? '')" :to="urlFormat(item.link)" target="_blank">{{
                                    $t(item.linkTitle ?? '')
                                }}</NuxtLink>
                            </li>
                        </ul>
                    </div>
                </div>
                <div class="center_con acea-row">
                    <div class="left_con">
                        <div class="global_shoppe">
                            <NuxtLink
                                v-if="module.picList2[0]"
                                :title="module.picList2[0]?.picTitle ?? ''"
                                :to="urlFormat(module.picList2[0].picLink)"
                                target="_blank"
                                class="global_pic"
                            >
                                <p class="pro_tit">{{ module.picList2[0]?.picTitle ?? "" }}</p>
                                <p class="pro_sub_tit">{{ $t(module.picList2[0]?.picDesc ?? "") }}</p>
                                <el-image class="img" :src="imageFormat(module.picList2[0]?.picUrl)" :alt="module.picList2[0]?.picTitle ?? ''" />
                            </NuxtLink>
                        </div>
                        <div class="pro_item_wrap">
                            <div class="pro_shoppe">
                                <NuxtLink
                                    v-if="module.picList2[1]"
                                    :title="module.picList2[1]?.picTitle ?? ''"
                                    :to="urlFormat(module.picList2[1].picLink)"
                                    target="_blank"
                                >
                                    <p class="pro_tit">{{ module.picList2[1]?.picTitle ?? '' }}</p>
                                    <p class="pro_sub_tit">{{ $t(module.picList2[1]?.picDesc ?? '') }}</p>
                                    <el-image class="img" :src="imageFormat(module.picList2[1]?.picUrl)" :alt="module.picList2[1]?.picTitle" />
                                </NuxtLink>
                            </div>
                            <div class="pro_shoppe">
                                <NuxtLink
                                    v-if="module.picList2[2]"
                                    :title="module.picList2[2]?.picTitle ?? ''"
                                    :to="urlFormat(module.picList2[2].picLink)"
                                    target="_blank"
                                >
                                    <p class="pro_tit">{{ module.picList2[2]?.picTitle ?? '' }}</p>
                                    <p class="pro_sub_tit">{{ $t(module.picList2[2]?.picDesc ?? '') }}</p>
                                    <el-image class="img" :src="imageFormat(module.picList2[2]?.picUrl)" :alt="module.picList2[2]?.picTitle ?? ''" />
                                </NuxtLink>
                            </div>
                        </div>
                    </div>
                    <div class="right_con">
                        <div class="pro_shoppe">
                            <NuxtLink
                                v-if="module.picList2[3]"
                                :title="module.picList2[3]?.picTitle ?? ''"
                                :to="urlFormat(module.picList2[3].picLink)"
                                target="_blank"
                            >
                                <p class="pro_tit">{{ module.picList2[3]?.picTitle ?? '' }}</p>
                                <p class="pro_sub_tit">{{ $t(module.picList2[3]?.picDesc ?? '') }}</p>
                                <el-image class="img" :src="imageFormat(module.picList2[3]?.picUrl)" :alt="module.picList2[3]?.picTitle ?? ''" />
                            </NuxtLink>
                        </div>
                        <div class="pro_shoppe">
                            <NuxtLink
                                v-if="module.picList2[4]"
                                :title="module.picList2[4]?.picTitle ?? ''"
                                :to="urlFormat(module.picList2[4].picLink)"
                                target="_blank"
                            >
                                <p class="pro_tit">{{ module.picList2[4]?.picTitle ?? '' }}</p>
                                <p class="pro_sub_tit">{{ $t(module.picList2[4]?.picDesc ?? '') }}</p>
                                <el-image class="img" :src="imageFormat(module.picList2[4]?.picUrl)" :alt="module.picList2[4]?.picTitle ?? ''" />
                            </NuxtLink>
                        </div>
                    </div>
                </div>
            </div>
            <div class="global_category">
                <ul class="acea-row">
                    <li :class="'jk_' + (index + 1)" v-for="(item, index) in module.picList3" :key="index">
                        <NuxtLink :title="item.picTitle ?? ''" :to="urlFormat(item.picLink)" target="_blank" class="global_a">
                            <p class="pro_tit">{{ item.picTitle ?? '' }}</p>
                            <p class="pro_sub_tit">{{ $t(item.picDesc ?? '') }}</p>
                            <p class="pro_color"></p>
                            <el-image class="img" :src="imageFormat(item.picUrl)" :alt="item.picTitle ?? ''" />
                        </NuxtLink>
                    </li>
                </ul>
            </div>
        </div>
    </div>
</template>
<script lang="ts" setup>
import { ref, computed } from "vue";
const props = defineProps({
    modelValue: { type: Object }
});

interface ModulePicListType {
    picUrl?: string;
    picThumb?: string;
    picLink?: {
        data?: object;
        link?: string;
        title?: string;
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
interface PcCustomAd2Module {
    picList?: ModulePicListType | [];
    picList2: ModulePicListType[];
    picList3: ModulePicListType[];
    linkList: ModuleLinkListType[];
}
const module = ref(props.modelValue as PcCustomAd2Module);
const mainPicture = computed(() => Array.isArray(module.value.picList) ? undefined : module.value.picList);
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
.mod_global_imported {
    margin-top: 25px;
}
.mod_global_imported h4 {
    padding: 12px;
    color: #645144;
    font-size: 20px;
    line-height: 20px;
    font-weight: 700;
}
.mod_global_imported .global_food {
    width: 930px;
    height: 450px;
}
.mod_global_imported .left_banner {
    position: relative;
}
.mod_global_imported .left_banner .banner_link {
    position: absolute;
    bottom: 10px;
    left: 20px;
    z-index: 1;
    width: 300px;
}
.mod_global_imported .left_banner li a {
    display: block;
    margin: 0 10px 10px 0;
    width: 83px;
    height: 28px;
    border: 1px solid rgba(255, 255, 255, 0.2);
    background-color: rgba(255, 255, 255, 0.1);
    color: #fff;
    text-align: center;
    font-size: 14px;
    line-height: 28px;
}
.mod_global_imported .left_banner li a:hover {
    border: 1px solid rgba(255, 255, 255, 0.35);
    background-color: rgba(255, 255, 255, 0.25);
    color: #fff !important;
}
.mod_global_imported .center_con {
    padding: 5px 5px 5px 0;
    width: 609px;
}
.mod_global_imported .center_con a {
    -webkit-transition: -webkit-transform 0.4s ease;
    -moz-transition:
        transform 0.4s ease,
        -moz-transform 0.4s ease;
    -o-transition: -o-transform 0.4s ease;
    transition: -webkit-transform 0.4s ease;
    transition: transform 0.4s ease;
    transition:
        transform 0.4s ease,
        -webkit-transform 0.4s ease,
        -moz-transform 0.4s ease,
        -o-transform 0.4s ease;
}
.mod_global_imported .center_con a:hover {
    z-index: 105;
    -webkit-box-shadow: 0 0 5px #bdbcbc;
    -moz-box-shadow: 0 0 5px #bdbcbc;
    box-shadow: 0 0 5px #bdbcbc;
    -webkit-transform: scale(1.05);
    -moz-transform: scale(1.05);
    -o-transform: scale(1.05);
    transform: scale(1.05);
    -ms-transform: scale(1.05);
}
.mod_global_imported .global_pic {
    position: relative;
    display: block;
    margin-bottom: 1px;
    width: 405px;
    height: 220px;
    background-color: #fff;
}
.mod_global_imported .global_shoppe a {
    position: relative;
    padding: 20px 0 0 20px;
    box-sizing: border-box;
    .pro_tit {
        position: relative;
        z-index: 1;
        overflow: hidden;
        width: 150px;
        height: 24px;
        color: #333;
    }
    .pro_sub_tit {
        position: relative;
        z-index: 1;
        overflow: hidden;
        width: 150px;
        height: 24px;
        color: #999;
    }
}
.mod_global_imported .global_pic .img {
    width: 405px;
    height: 220px;
    position: absolute;
    left: 0;
    top: 0;
}
.mod_global_imported .pro_item_wrap {
    display: flex;
    width: 406px;
}
.mod_global_imported .pro_item_wrap .pro_shoppe {
    margin-right: 1px;
    width: 202px;
}
.mod_global_imported .pro_shoppe {
    padding-bottom: 1px;
}
.mod_global_imported .pro_shoppe a {
    position: relative;
    display: block;
    padding: 20px 0 0 20px;
    height: 200px;
    background-color: #fff;
    font-size: 14px;
    line-height: 24px;
}
.mod_global_imported .pro_shoppe .pro_tit {
    overflow: hidden;
    width: 150px;
    height: 24px;
    color: #333;
}
.mod_global_imported .pro_shoppe .pro_sub_tit {
    overflow: hidden;
    width: 150px;
    height: 24px;
    color: #999;
}
.mod_global_imported .pro_shoppe .img {
    position: absolute;
    right: 10px;
    bottom: 10px;
    width: 140px;
    height: 140px;
}
.mod_global_imported .right_con {
    width: 203px;
}
.mod_global_imported .global_category {
    width: 260px;
    height: 450px;
}
.mod_global_imported .global_category li {
    position: relative;
}
.mod_global_imported .global_category .jk_1 {
    background-color: #ffe6ee;
}
.mod_global_imported .global_category .jk_2 {
    background-color: #fff3e4;
}
.mod_global_imported .global_category .jk_3 {
    background-color: #e9fbf0;
}
.mod_global_imported .global_category .jk_4 {
    background-color: #f9eee9;
}
.mod_global_imported .global_category .jk_5 {
    background-color: #f0f8ff;
}
.mod_global_imported .global_category .jk_6 {
    background-color: #e2f8f2;
}
.mod_global_imported .global_category .global_a {
    position: relative;
    display: block;
    width: 118px;
    height: 138px;
    overflow: hidden;
    padding: 12px 0 0 12px;
    line-height: 26px;
}
.mod_global_imported .global_category .pro_tit {
    height: 26px;
    overflow: hidden;
    font-size: 14px;
}
.mod_global_imported .global_category .jk_1 .pro_tit {
    color: #f06291;
}
.mod_global_imported .global_category .jk_2 .pro_tit {
    color: #ee9832;
}
.mod_global_imported .global_category .jk_3 .pro_tit {
    color: #54a472;
}
.mod_global_imported .global_category .jk_4 .pro_tit {
    color: #ad7d62;
}
.mod_global_imported .global_category .jk_5 .pro_tit {
    color: #468bcb;
}
.mod_global_imported .global_category .jk_6 .pro_tit {
    color: #54a472;
}
.mod_global_imported .global_category .pro_sub_tit {
    position: relative;
    z-index: 105;
    height: 26px;
    overflow: hidden;
    color: #757575;
}
.mod_global_imported .global_category .pro_color {
    width: 20px;
    height: 3px;
    overflow: hidden;
    font-size: 0;
    line-height: 0;
}
.mod_global_imported .global_category .jk_1 .pro_color {
    background-color: #f06291;
}
.mod_global_imported .global_category .jk_2 .pro_color {
    background-color: #ee9832;
}
.mod_global_imported .global_category .jk_3 .pro_color {
    background-color: #54a472;
}
.mod_global_imported .global_category .jk_4 .pro_color {
    background-color: #ad7d62;
}
.mod_global_imported .global_category .jk_5 .pro_color {
    background-color: #468bcb;
}
.mod_global_imported .global_category .jk_6 .pro_color {
    background-color: #54a472;
}
.mod_global_imported .global_category .img {
    position: absolute;
    right: -5px;
    bottom: 0;
    width: 100px;
    height: 100px;
    -webkit-transition: -webkit-transform 0.4s ease 0s;
    transition: -webkit-transform 0.4s ease 0s;
    -o-transition: -o-transform 0.4s ease 0s;
    -moz-transition:
        transform 0.4s ease 0s,
        -moz-transform 0.4s ease 0s;
    transition: transform 0.4s ease 0s;
    transition:
        transform 0.4s ease 0s,
        -webkit-transform 0.4s ease 0s,
        -moz-transform 0.4s ease 0s,
        -o-transform 0.4s ease 0s;
}
.mod_global_imported .global_category .global_a:hover .img {
    -webkit-transform: translate3d(-5px, 0, 0);
    -moz-transform: translate3d(-5px, 0, 0);
    transform: translate3d(-5px, 0, 0);
}
</style>
