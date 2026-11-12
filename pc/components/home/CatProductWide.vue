<template>
    <div class="floor-wide__box">
        <div class="mod_index_floor">
            <div class="left_con" :style="leftConStyle">
                <h4>
                    <NuxtLink
                        class="acea-row row-center-wrapper"
                        :title="modelValue.title"
                        :to="urlFormat({ path: 'list', seoLink: modelValue.seoLink, id: modelValue.categoryId })"
                        target="_blank"
                    >
                        <div class="">{{ modelValue.title }}</div>
                        <i class="iconfont-pc icon-yuanjiaojiantouyou f10 ml10"></i>
                    </NuxtLink>
                </h4>
                <p class="en_tit">{{ $t(modelValue.enTitle) }}</p>
                <SwiperSpecialSwiper v-model="modelValue.picList" :activeColor="modelValue.color"></SwiperSpecialSwiper>
                <div class="comment acea-row row-middle">
                    <div class="comment_left" :style="'border-color: ' + modelValue.color">
                        <i class="iconfont-pc icon-pinlun"></i>
                    </div>
                    <div class="comment_list">
                        <ul>
                            <li class="ellipsis" v-for="(item, index) in modelValue?.linkList" :key="index">
                                <NuxtLink target="_blank" :title="$t(item.linkTitle)" :to="urlFormat(item.link)">{{ $t(item.linkTitle) }}</NuxtLink>
                            </li>
                        </ul>
                    </div>
                </div>
            </div>
            <div class="center_con">
                <div class="keywords">
                    <ul class="acea-row row-middle">
                        <li v-for="(item, index) in modelValue?.childCategoryList">
                            <template v-if="index < 8">
                                {{ index !== 0 && index < 8 ? "|" : "" }}
                                <NuxtLink
                                    v-if="index < 8"
                                    class="a-hot"
                                    :title="item.categoryName"
                                    :to="urlFormat({ path: 'list', seoLink: item.seoLink, id: item.categoryId })"
                                    target="_blank"
                                >
                                    {{ item.categoryName }}</NuxtLink
                                >
                            </template>
                        </li>
                    </ul>
                </div>
                <div class="center_goods">
                    <div class="acea-row">
                        <div class="pro_wrap" v-for="(item, index) in modelValue?.picList2" :key="index">
                            <NuxtLink :title="item?.picTitle" target="_blank" :to="urlFormat(item?.picLink)" v-if="index < 6">
                                <p class="pro_tit">{{ item?.picTitle }}</p>
                                <p class="pro_sub_tit" :style="textColorStyle">{{ $t(item?.picDesc) }}</p>
                                <span class="pic-img-box--small">
                                    <el-image class="pro_image" lazy :src="productImage(item)" :alt="item?.productName || ''" />
                                </span>
                            </NuxtLink>
                        </div>
                    </div>
                </div>
            </div>
            <div class="hot_con">
                <p class="hot_tit">{{ $t("热门商品") }}</p>
                <div class="rank_list">
                    <ul>
                        <li v-for="(item, index) in list" :key="item.productId">
                            <NuxtLink
                                class="pro_img acea-row row-between"
                                :title="item.productName"
                                target="_blank"
                                :to="urlFormat({ path: 'product', seoLink: item.seoLink, sn: item.productSn })"
                            >
                                <span class="pro_image_wrap">
                                    <el-image class="pro_image" lazy :alt="item.productName" :src="productImage(item)" height="60px" width="60px" />
                                    <img
                                        v-if="item.productId && styleMap[item.productId]?.borderImage"
                                        class="product-style-border"
                                        :src="styleMap[item.productId].borderImage || undefined"
                                    />
                                </span>
                                <div class="rank_detail">
                                    <p class="pro_name line2">
                                        <span v-if="item.saleMode || item.productSaleMode" class="presale-text">{{ $t("预售") }}</span>
                                        <span v-if="item.saleMode || item.productSaleMode">|</span>
                                        {{ item.productName }}
                                    </p>
                                    <FormatPrice class="pro_price" v-model="item.productPrice"></FormatPrice>
                                </div>
                            </NuxtLink>
                        </li>
                    </ul>
                </div>
            </div>
        </div>
    </div>
</template>
<script lang="ts" setup>
import { computed } from "vue";
import { urlFormat } from "@/utils/format";
import { useComProParams, useProductStyle } from "@/hooks";

const props = defineProps({
    modelValue: {
        type: Object,
        default: () => ({})
    }
});

const { productList } = useComProParams({ productSelectType: 1, productIds: props.modelValue.productIds });

const list = computed(() => {
    return productList.value?.length > 5 ? productList.value.slice(0, 5) : productList.value;
});
const { styleMap } = useProductStyle(list);

const leftConStyle = {
    background: "linear-gradient(30deg, " + props.modelValue.color + " 0%, " + props.modelValue.color2 + " 100%)"
};
const textColorStyle = {
    color: props.modelValue.color
};
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

.acea-row.row-between {
    justify-content: space-between;
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

.line2 {
    word-break: break-all;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
}

// 通宽
.mod_index_floor {
    height: 430px;
    margin-top: 20px;
    background-color: #fff;
    display: flex;
    flex-direction: row;
}

.mod_index_floor .left_con {
    position: relative;
    width: 236px;
    height: 410px;
    padding-top: 20px;
    text-align: center;
}

.mod_index_floor .left_con {
    background: #f9ab57;
    background: linear-gradient(30deg, #f9ab57 0%, #f78a4b 100%);
}

.mod_index_floor .left_con h4 a {
    border-bottom: 1px solid #feeadd;
    width: 100px;
    padding: 0 10px 10px 15px;
    font-size: 20px;
    color: #fff;
    line-height: 20px;
    margin: auto;
}

.mod_index_floor .left_con .en_tit {    
    font-family: "微软雅黑";
    color: #fff;
    font-size: 14px;
    line-height: 24px;
    text-transform: uppercase;
}

.mod_index_floor .left_con .comment {
    position: absolute;
    bottom: 0;
    left: 0;
    width: 100%;
    height: 26px;
    overflow: hidden;
    padding: 8px 0;
    background-color: rgba(255, 255, 255, 0.2);
    line-height: 26px;
}

.mod_index_floor .left_con .comment .comment_left {
    border-right: 1px solid #f7d9b7;
}

.mod_index_floor .left_con .comment .comment_left {
    width: 42px;
    height: 26px;
    color: #fff;
}

.mod_index_floor .left_con .comment .comment_list {
    width: 173px;
    height: 26px;
    overflow: hidden;
    padding: 0 10px;
    text-align: left;
}

.mod_index_floor .left_con .comment .comment_list li {
    color: #fff;
}

.mod_index_floor .left_con .comment .comment_list {
    a {
        color: #fff;
    }

    a:hover {
        color: #fff !important;
    }
}

.mod_index_floor .center_con {
    width: 704px;
}

.mod_index_floor .center_con .keywords {
    height: 40px;
    overflow: hidden;
    border-right: 1px solid #f5f5f5;
    color: #aaa;
}

.mod_index_floor .center_con .keywords a.a-hot {
    display: inline-block;
    margin: 0 15px;
    line-height: 40px;
    color: #aaa;
}

.mod_index_floor .center_con .keywords a.a-hot:hover {
    color: var(--general);
}

.mod_index_floor .center_con .pro_wrap {
    box-sizing: border-box;
    position: relative;
    width: 233px;
    height: 194px;
    overflow: hidden;
    border-top: 1px solid #f5f5f5;
    border-right: 1px solid #f5f5f5;
}

.mod_index_floor .center_con .pro_wrap:nth-child(3n) {
    width: 235px;
}

.mod_index_floor .center_con .pro_wrap a {
    display: block;
    width: 100%;
    height: 174px;
    padding-top: 20px;
}

.mod_index_floor .center_con .pro_wrap a:hover {
    text-decoration: none;
}

.mod_index_floor .center_con .pro_wrap .pro_tit {
    position: relative;
    z-index: 105;
    height: 24px;
    overflow: hidden;
    max-width: 145px;
    margin-left: 20px;
    font-size: 14px;
    line-height: 24px;
    color: #333;
}

.mod_index_floor .center_con .pro_wrap .pro_sub_tit {
    position: relative;
    z-index: 105;
    height: 24px;
    overflow: hidden;
    max-width: 115px;
    margin-left: 20px;
    font-size: 14px;
    line-height: 24px;
}

.mod_index_floor .center_con .pro_wrap .pro_sub_tit {
    color: #f78b4b;
}

.mod_index_floor .center_con .pro_wrap .pic-img-box--small {
    position: absolute;
    right: 10px;
    bottom: 10px;
    z-index: 101;
    width: 130px;
    height: 130px;
    display: block;
}

.mod_index_floor .center_con .pro_wrap .pic-img-box--small :deep(.pro_image) {
    width: 130px;
    height: 130px;
    transition:
        transform 0.4s ease 0s,
        -webkit-transform 0.4s ease 0s,
        -moz-transform 0.4s ease 0s,
        -o-transform 0.4s ease 0s;
}

.mod_index_floor .center_con .pro_wrap a:hover .pic-img-box--small :deep(.pro_image) {
    -webkit-transform: translate3d(-5px, 0, 0);
    -moz-transform: translate3d(-5px, 0, 0);
    transform: translate3d(-5px, 0, 0);
}

.mod_index_floor .hot_con {
    width: 250px;
    height: 430px;
    overflow: hidden;
    overflow-y: auto;
    &::-webkit-scrollbar {
        display: none;
    }
}

.mod_index_floor .hot_con .hot_tit {
    margin-top: 15px;
    color: #666;
    font-weight: bold;
    font-size: 14px;
    line-height: 20px;
    text-align: center;
}

.mod_index_floor .hot_con .rank_list {
    padding: 0 20px;
}

.mod_index_floor .hot_con .rank_list li {
    margin-top: 15px;
    height: 60px;
}

.mod_index_floor .hot_con .pro_image_wrap {
    position: relative;
    display: inline-block;
    width: 60px;
    height: 60px;
    flex-shrink: 0;
}

.mod_index_floor .hot_con .pro_image {
    position: relative;
    width: 60px;
    height: 60px;
}

.product-style-border {
    position: absolute;
    left: 0;
    top: 0;
    width: 100%;
    height: 100%;
    pointer-events: none;
    object-fit: fill;
}

.mod_index_floor .hot_con .rank_detail {
    width: 138px;
}

.mod_index_floor .hot_con .rank_detail .pro_name {
    height: 40px;
    overflow: hidden;
    line-height: 20px;
}

.presale-text {
    color: var(--general);
    margin-right: 4px;
    font-weight: 500;
}

.mod_index_floor .hot_con .rank_detail .pro_price {
    margin-top: 5px;
    color: var(--general);
}
</style>
