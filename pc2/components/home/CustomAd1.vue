<template>
    <div class="mod_on_new">
        <div class="left_banner">
            <NuxtLink
                v-if="modelValue?.picList[0]"
                :title="modelValue?.picList[0]?.picTitle ?? ''"
                :to="urlFormat(modelValue?.picList[0]?.picLink)"
                target="_blank"
            >
                <el-image
                    class="img"
                    lazy
                    :src="imageFormat(modelValue?.picList[0]?.picUrl)"
                    :alt="modelValue?.picList[0]?.picTitle ?? ''"
                    style="background: #eee"
                />
            </NuxtLink>
        </div>
        <div class="brand_buy mr10">
            <h4>
                <em>
                    <u class="left_triangle"></u>
                    <u class="right_triangle"></u>
                    <i class="iconfont-pc icon-jingpin"></i>
                    <span>{{ $t("每日推荐") }}</span>
                </em>
            </h4>
            <div class="acea-row row-middle">
                <NuxtLink
                    v-if="modelValue?.picList2[0]"
                    :title="modelValue?.picList2[0]?.picTitle ?? ''"
                    :to="urlFormat(modelValue?.picList2[0]?.picLink)"
                    class="brand_banner"
                    target="_blank"
                >
                    <el-image class="img" lazy :src="imageFormat(modelValue?.picList2[0]?.picUrl)" :alt="modelValue?.picList2[0]?.picTitle ?? ''" />
                </NuxtLink>
                <div class="brand_right">
                    <ul>
                        <template v-for="(item, index) in modelValue?.picList2" :key="index">
                            <li v-if="index != 0">
                                <NuxtLink :title="item?.picTitle ?? ''" :to="urlFormat(item?.picLink)" target="_blank">
                                    <p class="pro_tit">{{ item?.picTitle ?? "" }}</p>
                                    <p class="pro_sub_tit">{{ $t(item?.picDesc ?? "") }}</p>
                                    <el-image
                                        class="img"
                                        lazy
                                        :src="imageFormat(item.picUrl)"
                                        :alt="item?.picTitle ?? ''"
                                        style="width: 90px; height: 90px"
                                    />
                                </NuxtLink>
                            </li>
                        </template>
                    </ul>
                </div>
            </div>
        </div>
        <div class="brand_buy life_pie">
            <h4>
                <em>
                    <u class="left_triangle"></u>
                    <u class="right_triangle"></u>
                    <i class="iconfont-pc icon-jingpintuijian"></i>
                    <span>{{ $t("热销精品") }}</span>
                </em>
            </h4>
            <div class="acea-row row-middle" style="height: 230px">
                <NuxtLink
                    v-if="modelValue?.picList3 && modelValue?.picList3[0]"
                    :title="modelValue?.picList3[0]?.picTitle ?? ''"
                    :to="urlFormat(modelValue?.picList3[0]?.picLink)"
                    class="brand_banner"
                    target="_blank"
                >
                    <el-image class="img" lazy :src="imageFormat(modelValue?.picList3[0]?.picUrl)" :alt="modelValue?.picList3[0]?.picTitle ?? ''" />
                    <p class="pro_tit">{{ modelValue?.picList3[0]?.picTitle ?? "" }}</p>
                    <p class="pro_sub_tit">{{ $t(modelValue?.picList3[0]?.picDesc ?? "") }}</p>
                </NuxtLink>
                <div class="brand_right">
                    <ul>
                        <li>
                            <NuxtLink :title="modelValue?.picList3[1]?.picTitle ?? ''" :to="urlFormat(modelValue?.picList3[1]?.picLink)" target="_blank">
                                <p class="pro_tit">{{ modelValue?.picList3[1]?.picTitle ?? "" }}</p>
                                <p class="pro_sub_tit">{{ $t(modelValue?.picList3[1]?.picDesc ?? "") }}</p>
                                <el-image
                                    class="img"
                                    lazy
                                    :src="imageFormat(modelValue?.picList3[1]?.picUrl)"
                                    :alt="modelValue?.picList3[1]?.picTitle ?? ''"
                                    style="width: 90px; height: 90px"
                                />
                            </NuxtLink>
                        </li>
                        <li>
                            <NuxtLink :title="modelValue?.picList3?.[2]?.picTitle ?? ''" :to="urlFormat(modelValue?.picList3[2]?.picLink)" target="_blank">
                                <p class="pro_tit">{{ modelValue?.picList3[2]?.picTitle ?? "" }}</p>
                                <p class="pro_sub_tit">{{ $t(modelValue?.picList3[2]?.picDesc ?? "") }}</p>
                                <el-image
                                    class="img"
                                    lazy
                                    :src="imageFormat(modelValue?.picList3[2]?.picUrl)"
                                    :alt="modelValue?.picList3[2]?.picTitle ?? ''"
                                    style="width: 90px; height: 90px"
                                />
                            </NuxtLink>
                        </li>
                    </ul>
                </div>
            </div>
            <div class="life_pic">
                <NuxtLink :title="modelValue?.picList3[3]?.picTitle ?? ''" :to="urlFormat(modelValue?.picList3[3]?.picLink)" target="_blank" class="shp_a">
                    <el-image class="img" lazy :src="imageFormat(modelValue?.picList3[3]?.picUrl)" :alt="modelValue?.picList3[3]?.picTitle ?? ''" />
                    <p class="shp_p shp_pp">{{ modelValue?.picList3[3]?.picTitle ?? "" }}</p>
                    <p class="go shp_p">GO<em class="index_iconfont"> &gt;&gt;</em></p>
                </NuxtLink>
                <NuxtLink :title="modelValue?.picList3[4]?.picTitle ?? ''" :to="urlFormat(modelValue?.picList3[4]?.picLink)" target="_blank" class="shp_a">
                    <el-image class="img" lazy :src="imageFormat(modelValue?.picList3[4]?.picUrl)" :alt="modelValue?.picList3[4]?.picTitle ?? ''" />
                    <p class="shp_p shp_pp">{{ modelValue?.picList3[4]?.picTitle ?? "" }}</p>
                    <p class="go shp_p">GO<em class="index_iconfont"> &gt;&gt;</em></p>
                </NuxtLink>
            </div>
        </div>
    </div>
</template>
<script lang="ts" setup>
import { ref, watch } from "vue";
const props = defineProps({
    modelValue: { type: Object }
});

interface ModulePicListType {
    picUrl?: string;
    picThumb?: string;
    picLink?: {
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
interface PcCatProductType {
    categoryId?: number;
    childCategoryList?: {
        categoryId: number;
        categoryName: string;
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

.mr10 {
    margin-right: 10px;
}

/*首页上新*/
.mod_on_new * {
    box-sizing: border-box;
}

.mod_on_new {
    display: flex;
    margin-top: 30px;
    flex-direction: row;
}

.mod_on_new .left_banner {
    overflow: hidden;
    margin-right: 10px;
    width: 270px;
    height: 400px;
}

.mod_on_new .left_banner .img {
    width: 270px;
    height: 400px;
}

.mod_on_new h4 {
    margin-bottom: 7px;
    height: 32px;
    color: #fff;
    text-align: center;
    font-weight: 400;
    line-height: 32px;
}

.mod_on_new h4 em {
    position: relative;
    top: -3px;
    display: inline-block;
    margin-top: -3px;
    width: 140px;
    height: 32px;
    font-size: 18px;
}

.mod_on_new h4 em i {
    margin-right: 8px;
    font-size: 20px;
}

.mod_on_new h4 em .left_triangle {
    position: absolute;
    top: 0;
    left: -4px;
    overflow: hidden;
    width: 0;
    height: 0;
    border-bottom: 4px solid #a67724;
    border-left: 4px solid transparent;
    font-size: 0;
    line-height: 0;
}

.mod_on_new h4 em .right_triangle {
    position: absolute;
    top: 0;
    right: -4px;
    overflow: hidden;
    width: 0;
    height: 0;
    border-right: 4px solid transparent;
    border-bottom: 4px solid #a67724;
    font-size: 0;
    line-height: 0;
}

.mod_on_new .brand_right {
    overflow: hidden;
    margin-top: 1px;
    padding: 0 15px;
    height: 360px;
    font-size: 14px;
    line-height: 24px;
}

.mod_on_new .brand_right .pro_tit {
    position: relative;
    z-index: 105;
    overflow: hidden;
    width: 95px;
    height: 24px;
    color: #333;
    line-height: 24px;
}

.mod_on_new .brand_right .pro_sub_tit {
    position: relative;
    z-index: 105;
    overflow: hidden;
    height: 24px;
    color: #999;
}

.mod_on_new .brand_right li {
    position: relative;
    margin-bottom: 10px;
    padding: 10px 5px 10px;
    height: 110px;
    border-bottom: 1px solid #f0f0f0;
}

.mod_on_new .brand_right li:last-child {
    margin-bottom: 0;
    border-bottom: 0;
}

.mod_on_new .brand_right li .img {
    position: absolute;
    right: 5px;
    bottom: 10px;
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

.mod_on_new .brand_right li a {
    display: block;
    height: 90px;
}

.mod_on_new .brand_right li a:hover .img {
    -webkit-transform: translate3d(-5px, 0, 0);
    -moz-transform: translate3d(-5px, 0, 0);
    transform: translate3d(-5px, 0, 0);
}

.mod_on_new .brand_buy {
    width: 450px;
    height: 400px;
    background: #fff url(/assets/images/index/group_bg.png) no-repeat center top;
}

.mod_on_new .brand_buy h4 {
    border-top: 1px solid #dfa747;
}

.mod_on_new .brand_buy h4 em {
    background: #dfa747;
}

.mod_on_new .brand_buy .brand_banner {
    position: relative;
    display: block;
    overflow: hidden;
    width: 225px;
    height: 360px;
    border-right: 1px solid #f0f0f0;
}

.mod_on_new .brand_buy .brand_banner .img {
    width: 225px;
    height: 360px;
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

.mod_on_new .brand_buy .brand_banner:hover .img {
    -webkit-transform: translate3d(-5px, 0, 0);
    -moz-transform: translate3d(-5px, 0, 0);
    transform: translate3d(-5px, 0, 0);
}

.mod_on_new .brand_buy .brand_right {
    width: 225px;
}

.mod_on_new .life_pie {
    width: 450px;
    height: 400px;
    background: #fff url(/assets/images/index/group_bg.png) no-repeat center top;
}

.mod_on_new .life_pie .brand_banner {
    height: 230px;
}

.mod_on_new .life_pie .brand_right {
    height: 230px;
}

.mod_on_new .life_pie h4 {
    position: relative;
    border-top: 1px solid #f39156;
}

.mod_on_new .life_pie h4 em {
    background: #f39156;
}

.mod_on_new .life_pie .brand_banner {
    position: relative;
    display: block;
    overflow: hidden;
    margin-left: 20px;
    padding-top: 8px;
    width: 205px;
    height: 210px;
    border-right: 1px solid #f0f0f0;
}

.mod_on_new .life_pie .brand_banner .img {
    position: absolute;
    right: -5px;
    bottom: 0;
    width: 210px;
    height: 210px;
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

.mod_on_new .life_pie .brand_banner .pro_tit {
    position: relative;
    z-index: 105;
    margin-top: 8px;
    color: #333;
    font-size: 16px;
}

.mod_on_new .life_pie .brand_banner .pro_sub_tit {
    position: relative;
    z-index: 105;
    overflow: hidden;
    margin-top: 5px;
    width: 85px;
    height: 40px;
    color: #999;
    font-size: 14px;
    line-height: 20px;
}

.mod_on_new .life_pie .life_pic {
    display: flex;
    padding: 10px 10px 13px 14px;
}

.mod_on_new .life_pie .life_pic a.shp_a {
    position: relative;
    display: block;
    overflow: hidden;
    padding: 10px 0 0 12px;
    width: 205px;
    height: 105px;
}

.mod_on_new .life_pie .life_pic a.shp_a:first-child {
    margin-right: 12px;
}

.mod_on_new .life_pie .life_pic a:hover .img {
    -webkit-transform: translate3d(-5px, 0, 0);
    -moz-transform: translate3d(-5px, 0, 0);
    transform: translate3d(-5px, 0, 0);
}

.mod_on_new .life_pie .life_pic .img {
    position: absolute;
    top: 0;
    right: -5px;
    width: 210px;
    height: 105px;
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

.mod_on_new .life_pie .life_pic p.shp_p {
    position: relative;
    z-index: 105;
    height: 20px;
    color: #fb6233;
    line-height: 20px;
}

.mod_on_new .life_pie .life_pic p.shp_pp {
    position: relative;
    z-index: 105;
    height: 20px;
    color: #fb6233;
    font-size: 14px;
    line-height: 20px;
}

.mod_on_new .life_pie .life_pic .go {
    font-size: 12px;
}
</style>
