<template>
    <div v-if="productList.length > 0" class="simple-goodsbox">
        <div class="head-title">
            <div class="tit">{{ modelValue.title }}</div>
            <NuxtLink :to="urlFormat(modelValue.moreLink)" class="more" target="_blank">
                {{ $t("更多") }}
                <svg width="15" height="17" viewBox="0 0 1024 1024">
                    <path
                        d="M329.354 820.029c-6.006 6.002-9.38 14.144-9.38 22.635s3.374 16.633 9.38 22.635c12.508 12.483 32.761 12.483 45.269 0l341.333-340.821c6.003-5.994 9.376-14.13 9.376-22.613s-3.373-16.619-9.376-22.613l-341.333-341.888c-12.508-12.483-32.761-12.483-45.269 0-6.006 6.002-9.38 14.144-9.38 22.635s3.374 16.633 9.38 22.635l318.72 319.232-318.72 318.165z"
                        fill="#000000"
                        fill-opacity="0.2"
                    ></path>
                </svg>
            </NuxtLink>
        </div>
        <div class="contnt" :class="{ 'product-small__wrap': modelValue.baseStyle == 2 }">
            <div class="left" v-if="modelValue.baseStyle == 2">
                <template v-if="modelValue.picList">
                    <NuxtLink :to="urlFormat(modelValue.picList[0]?.picLink)" class="imgs" target="_blank">
                        <img :src="imageFormat(modelValue.picList[0]?.picUrl)" :alt="modelValue.title || ''" />
                    </NuxtLink>
                </template>
            </div>
            <div class="right">
                <div class="goods-wrap">
                    <template v-for="item in productList">
                        <NuxtLink :to="urlFormat({ path: 'product', seoLink: item.seoLink, sn: item.productSn })" class="item" target="_blank">
                            <div class="pic">
                                <img :src="productImage(item)" :alt="item.productName || ''"  />
                                <img
                                    v-if="styleMap[item.productId]?.borderImage"
                                    class="product-style-border"
                                    :src="styleMap[item.productId].borderImage"
                                    :alt="item.productName || ''"
                                />
                            </div>
                            <div class="info">
                                <div class="name">{{ item.productName }}</div>
                                <div class="desc">{{ $t(item.productBrief ?? "") }}</div>
                                <div class="price-box">
                                    <div class="newprice">
                                        <FormatPrice v-model="item.productPrice" :fontStyle="{ fontSize: '16px' }"></FormatPrice>
                                    </div>
                                    <template v-if="item.marketPrice && Number(item.marketPrice) > 0">
                                        <div class="oldprice">
                                            <FormatPrice v-model="item.marketPrice" :fontStyle="{ textDecoration: 'line-through' }"></FormatPrice>
                                        </div>
                                    </template>
                                </div>
                            </div>
                        </NuxtLink>
                    </template>
                </div>
            </div>
        </div>
    </div>
</template>
<script lang="ts" setup>
import { urlFormat } from "~/utils/format";
import { useComProParams, useProductStyle } from "@/hooks";

const props = defineProps({
    moduleIndex: { type: Number, default: 0 },
    modelValue: { type: Object, default: {} }
});

const { productList } = useComProParams(props.modelValue.products);
const { styleMap } = useProductStyle(productList);
</script>
<style lang="less" scoped>
.simple-goodsbox {
    position: relative;
    width: 100%;
    margin-top: 20px;
    .head-title {
        display: flex;
        justify-content: space-between;
        align-items: center;
        margin: 15px 0;
        .tit {
            font-size: 24px;
            line-height: 32px;
            color: #333;
            font-weight: 400;
        }
        .more {
            display: flex;
            align-items: center;
            font-size: 14px;
            line-height: 19px;
            color: #666;
        }
    }
    .contnt {
        position: relative;
        &.product-small__wrap {
            display: flex;
            flex-direction: row;
            .left {
                width: 230px;
                flex-shrink: 0;
                margin-right: 12px;
                .imgs {
                    display: block;
                    width: 100%;
                    height: 100%;
                    transition: all 0.4s ease 0s;
                    &:hover {
                        box-shadow: rgba(0, 0, 0, 0.1) 0px 0px 36px;
                        transform: scale(1.005);
                    }
                    img {
                        width: 100%;
                        object-fit: contain;

                    }
                }
            }
            .right {
                width: 950px;
            }
            .goods-wrap {
                .pic {
                    height: 210px;
                }
            }
        }
    }
    .goods-wrap {
        position: relative;
        display: grid;
        grid-template-columns: repeat(4, minmax(0, 1fr));
        column-gap: 12px;
        row-gap: 12px;
        .item {
            background-color: #fff;
            transition: all 0.4s ease 0s;
            &:hover {
                box-shadow: rgba(0, 0, 0, 0.1) 0px 0px 36px;
                transform: translateY(-4px);
            }
            .pic {
                position: relative;
                display: flex;
                justify-content: center;
                align-items: center;
                height: 253px;
                img:first-of-type {
                    width: 180px;
                    height: 180px;
                }
                .product-style-border {
                    position: absolute;
                    left: 50%;
                    top: 50%;
                    width: 180px;
                    height: 180px;
                    margin-left: -90px;
                    margin-top: -90px;
                    z-index: 5;
                    pointer-events: none;
                    object-fit: fill;
                }
            }
            .info {
                padding: 10px 24px 24px 24px;
                display: flex;
                flex-direction: column;
                text-align: center;
                color: #333;
                white-space: nowrap;
                .name {
                    font-size: 14px;
                    line-height: 19px;
                    overflow: hidden;
                    text-overflow: ellipsis;
                }
                .desc {
                    font-size: 13px;
                    line-height: 18px;
                    margin-top: 4px;
                    opacity: 0.5;
                    overflow: hidden;
                    text-overflow: ellipsis;
                }
                .price-box {
                    margin-top: 12px;
                    display: flex;
                    justify-content: center;
                    align-items: center;
                    .newprice {
                        font-size: 16px;
                        line-height: 21px;
                        font-weight: 700;
                    }
                    .oldprice {
                        font-size: 12px;
                        line-height: 16px;
                        margin-left: 6px;
                        opacity: 0.5;
                        text-decoration: line-through;
                    }
                }
            }
        }
    }
}
</style>
