<template>
    <TotalContainer :module="module.frame" class="module-ad-con">
        <div class="productgrouping">
            <template v-if="module.groupTitle.showTitle || module.groupTitle.showSubTitle">
                <div class="tablist">
                    <template v-for="(item, index) in tabList" :key="index">
                        <div class="tablist-item" @click="switchTab(index)">
                            <div class="tab-text">
                                <div class="nav-item">
                                    <template v-if="index > 0">
                                        <div class="line" :style="`border-color:${module.groupTitle.splitLineColor}`" />
                                    </template>
                                    <div class="nav-item-box" :style="`padding: 0px ${module.groupTitle.marginLeftRight}px;`">
                                        <template v-if="module.groupTitle.showTitle">
                                            <div
                                                class="tab-title"
                                                :style="
                                                    tablistItemStyle.title + `${index === currentIndex ? `color:${module.groupTitle.activeTitleColor};` : ''}`
                                                "
                                            >
                                                {{ item.title }}
                                            </div>
                                        </template>
                                        <template v-if="item.subTitle && module.groupTitle.showSubTitle">
                                            <div
                                                class="tab-sub"
                                                :style="
                                                    tablistItemStyle.subTitle +
                                                    `${
                                                        index === currentIndex
                                                            ? `color:${module.groupTitle.activeSubTitleColor};background-image: linear-gradient(${
                                                                  gradientMap[module.groupTitle.subTitleGradientType]
                                                              }, ${module.groupTitle?.gradientColorA}, ${module.groupTitle.gradientColorB});`
                                                            : ''
                                                    }`
                                                "
                                            >
                                                {{ $t(item.subTitle) }}
                                            </div>
                                        </template>
                                    </div>
                                </div>
                            </div>
                        </div>
                    </template>
                </div>
            </template>

            <div class="productlist">
                <div class="productlist-item">
                    <div class="item-box">
                        <template v-for="(item, index) in list" :key="index">
                            <div class="li" :style="`opacity: 1; width: ${module.listStyle < 5 ? 100 / module.listStyle : index % 3 === 0 ? 100 : 50}%;`">
                                <div
                                    class="goods-item"
                                    :style="goodsItemStyle"
                                    @click="item.productId && navigateTo(urlFormat({ path: 'product', seoLink: item.seoLink, id: item.productId }))"
                                >
                                    <div class="goods-img" :style="`padding-top: ${picScaleMap[module.picScale!]}`">
                                        <div
                                            class="img"
                                            :style="`border-radius: ${
                                                module.picBottomRadius ? `${module.cardRadius}px` : `${module.cardRadius}px ${module.cardRadius}px 0 0`
                                            };
                                                    background-size: ${module.picFillType};
                                                    background-image: url(${productImage(item)});`"
                                        />
                                        <img
                                            v-if="item.productId && styleMap[item.productId]?.borderImage"
                                            class="product-style-border"
                                            :src="styleMap[item.productId].borderImage"
                                            alt=""
                                        />
                                    </div>
                                    <div
                                        class="goods-info"
                                        :style="`text-align: left; padding: ${module.productColor.infoTopPadding}px ${module.productColor.infoLeftRightPadding}px ${module.productColor.infoBottomPadding}px`"
                                    >
                                        <div
                                            v-if="module.showProductName"
                                            class="goods-title"
                                            :style="`margin-top: 3px;
                                                    height: ${module.productNameRow * 18}px;
                                                    line-height: 18px;
                                                    -webkit-line-clamp: ${module.productNameRow};
                                                    font-weight: ${module.productNameWeight};
                                                    color: ${module.productColor.titleColor};
                                                    font-size:  ${module.productNameSize}px;`"
                                        >
                                            {{ item.productName }}
                                        </div>
                                        <div
                                            class="goods-price-box"
                                            :style="`margin-top: ${module.productPriceMarginTop}px; justify-content: left; display: flex`"
                                        >
                                            <div class="price-origin-sale" style="justify-content: left">
                                                <div class="price-origin-icon">
                                                    <div class="price-icon">
                                                        <el-image
                                                            v-if="module.productPricePic && module.productPricePic.picUrl"
                                                            class="icon"
                                                            :src="module.productPricePic.picUrl"
                                                            alt=""
                                                        />
                                                        <div v-if="module.showProductPrice" class="price">
                                                            <format-price
                                                                :font-style="{
                                                                    fontSize: module.productPriceSize + 'px',
                                                                    color: module.productColor.priceColor,
                                                                    fontWeight: module.productPriceWeight
                                                                }"
                                                                :currency-style="{
                                                                    fontSize: module.productPriceSize - 2 + 'px',
                                                                    color: module.productColor.priceColor,
                                                                    fontWeight: module.productPriceWeight
                                                                }"
                                                                :decimals-style="{
                                                                    fontSize: module.productPriceSize - 2 + 'px',
                                                                    color: module.productColor.priceColor,
                                                                    fontWeight: module.productPriceWeight
                                                                }"
                                                                :price-data="item.productPrice"
                                                            />
                                                        </div>
                                                        <div
                                                            v-if="module.crossedOutPrice && item.marketPrice && item.marketPrice > 0"
                                                            class="origin"
                                                            :style="`color: ${module.productColor.crossedOutPriceColor}`"
                                                        >
                                                            <format-price
                                                                :font-style="{ fontSize: '12px', 'text-decoration': 'line-through' }"
                                                                :price-data="item.marketPrice"
                                                            />
                                                        </div>
                                                    </div>
                                                </div>
                                            </div>
                                        </div>
                                    </div>
                                </div>
                            </div>
                        </template>
                    </div>
                    <div
                        v-if="module.bottomMoreBtn !== 'close' && module.productSource !== 'productGroup'"
                        class="more-box"
                        @click="urlFormat(tabList[currentIndex].titleLink) && navigateTo(urlFormat(tabList[currentIndex].titleLink))"
                    >
                        <div
                            v-if="module.bottomMoreBtnText && module.bottomMoreBtn === 'text'"
                            class="more-txt"
                            :style="`margin: 10px 3px; background-color:${module.productColor.moreBtnBgColor}; border-color:${module.productColor.moreBtnBorderColor}; color: ${module.productColor.moreBtnTextColor}`"
                        >
                            {{ $t(module.bottomMoreBtnText) }}
                        </div>
                    </div>
                </div>
            </div>
        </div>
    </TotalContainer>
</template>

<script setup lang="ts">
/**
 * PC端装修商品分组组件
 * - 支持 Tab 切换展示不同商品分组
 * - 数据来源支持：商品分组(productGroup)或手动配置(productGroups)
 * - 支持自定义标题样式、分隔线、渐变色等
 * - 底部可配置"查看更多"按钮跳转
 */
import { computed, ref, watchEffect } from "vue";
import { urlFormat, productImage } from "@/utils/format";
import { getProductList } from "@/api/home/home";
import { useProductStyle } from "~/hooks";
import TotalContainer from "~/components/shop/src/TotalContainer.vue";

const props = defineProps({
    module: {
        type: Object,
        default: () => ({})
    }
});

const gradientMap: { [key: string]: string } = {
    "1": "to right",
    "2": "to bottom",
    "3": "to bottom right",
    "4": "to bottom left"
};

const picScaleMap: { [key: string]: string } = {
    "3:2": "calc(100% / 3 * 2)",
    "1:1": "calc(100% / 1 * 1)",
    "3:4": "calc(100% / 3 * 4)",
    "16:9": "calc(100% / 16 * 9)"
};

const tablistItemStyle = computed(() => {
    return {
        title: `color: ${props.module.groupTitle.titleColor}; font-size: ${props.module.groupTitle.bigTitleFontSize}px;`,
        subTitle: `color: ${props.module.groupTitle.subTitleColor}; font-size: ${props.module.groupTitle.titleFontSize}px;border-radius:${props.module.groupTitle.subTitleRadius}px;`
    };
});

const goodsItemStyle = computed(() => {
    let str = `margin: ${props.module.cardMargin / 2}px;padding: ${props.module.cardPadding}px;border: ${
        props.module.productCardStyle == 3 ? 1 : 0
    }px solid rgb(238, 238, 238);border-radius: ${props.module.cardRadius}px;`;
    if (props.module.productCardStyle != 4) {
        str += `background-color:${props.module.productColor.backgroundColor};`;
    }
    if (props.module.productCardStyle == 2) {
        str += `box-shadow: rgba(93, 113, 127, 0.08) 0px 2px 8px;`;
    }
    return str;
});

const tabList = computed(() => {
    return props.module.productSource === "productGroup" ? props.module.groupList : props.module.productGroups;
});

const currentIndex = ref(0);
const list = ref<any>([]);
const { styleMap } = useProductStyle(list);

const switchTab = (index: number) => {
    currentIndex.value = index;
};

const currentGroupData = ref<any>({});
const getGroupProductList = async () => {
    try {
        const result = await getProductList(currentGroupData.value);
        list.value = result.records;
    } catch (error) {
        console.error(error);
    }
};

watchEffect(() => {
    currentGroupData.value = {};
    if (props.module.productSource && props.module.productSource === "productGroup") {
        if (props.module.groupList.length > 0) {
            const { productGroupId, productNum, productNumType } = props.module.groupList[currentIndex.value];
            currentGroupData.value.productGroupId = productGroupId;
            currentGroupData.value.size = productNumType === "number" ? productNum : 100;
            getGroupProductList();
        }
    } else {
        if (props.module.productGroups && props.module.productGroups.length > 0) {
            currentGroupData.value.ids = props.module.productGroups[currentIndex.value].productList.map((item: any) => item.productId).join(",");
            getGroupProductList();
        }
    }
});
</script>

<style lang="less" scoped>
.module-ad-con {
    background-size: cover;
    background-repeat: no-repeat;
    background-position: center top;
    position: relative;
}

.tablist {
    overflow-x: auto;
    overflow-y: hidden;
    box-sizing: initial;
    height: 100%;
    position: relative;
    display: flex;

    &::-webkit-scrollbar {
        display: none;
        height: 0;
    }

    .tablist-item {
        padding: 0;
        flex: 1 0 auto;
        cursor: pointer;

        &.active {
            color: #323233;
            font-weight: 500;
        }

        .tab-text {
            width: 100%;

            .nav-item {
                text-align: center;
                overflow: hidden;
                position: relative;

                .line {
                    content: "";
                    border-right: #dadada solid 1px;
                    position: absolute;
                    left: 0;
                    top: 0;
                    bottom: 0;
                    -webkit-transform: scale(0.5);
                    transform: scale(0.5);
                }

                .nav-item-box {
                    display: inline-grid;

                    .tab-title {
                        margin: 0;
                        font-size: 18px;
                        font-weight: 700;
                        color: #333333;
                    }

                    .tab-sub {
                        margin-top: 5px;
                        padding: 3px 8px;
                        font-size: 14px;
                        color: #454545;
                    }
                }
            }
        }
    }
}

.productlist {
    position: relative;
    display: flex;
    width: 100%;
    height: 100%;

    .productlist-item {
        margin-top: 5px;
        flex-shrink: 0;
        box-sizing: border-box;
        width: 100%;

        .item-box {
            display: flex;
            flex-wrap: wrap;
            flex-direction: row;
            align-items: stretch;

            .goods-item {
                display: block;
                overflow: hidden;
                cursor: pointer;

                .goods-img {
                    position: relative;
                    max-height: 500px;
                    overflow: hidden;
                    width: 100%;
                    height: 0;
                    background-repeat: no-repeat;
                    background-position: center;

                    .img {
                        position: absolute;
                        top: 0;
                        right: 0;
                        bottom: 0;
                        left: 0;
                        background-repeat: no-repeat;
                        background-position: center;
                        background-size: 100% auto;
                        overflow: hidden;
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
                }

                .goods-info {
                    padding: 0 5px;
                    flex-grow: 1;
                    flex-shrink: 1;
                    display: flex;
                    -webkit-box-orient: vertical;
                    flex-direction: column;
                    overflow: hidden;
                    font-size: 12px;

                    .goods-title {
                        line-height: 18px;
                        font-size: 14px;
                        overflow: hidden;
                        display: -webkit-box;
                        -webkit-line-clamp: 1;
                        line-clamp: 1;
                        -webkit-box-orient: vertical;
                    }

                    .goods-price-box {
                        display: flex;
                        position: relative;
                        flex-wrap: nowrap;
                        align-items: center;
                        justify-content: space-between;

                        .price-origin-sale {
                            display: flex;
                            align-items: baseline;
                            flex-wrap: wrap;
                            overflow: hidden;
                            flex: 1;

                            .price-origin-icon {
                                margin: 3px 0;
                                display: flex;
                                flex-wrap: wrap;
                                align-items: baseline;
                                overflow: hidden;

                                .price-icon {
                                    display: flex;
                                    flex-wrap: wrap;
                                    align-items: self-end;
                                    overflow: hidden;

                                    .icon {
                                        height: 14px;
                                        margin-right: 3px;
                                        display: inline-block;
                                    }

                                    .price {
                                        margin-right: 5px;
                                        text-overflow: ellipsis;
                                        white-space: nowrap;
                                        overflow: hidden;
                                        display: flex;
                                        -webkit-box-align: baseline;
                                        align-items: baseline;
                                    }

                                    .origin {
                                        margin-right: 10px;
                                        text-overflow: ellipsis;
                                        white-space: nowrap;
                                        overflow: hidden;
                                        text-decoration: line-through;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    .more-box {
        display: flex;
        align-items: center;
        justify-content: center;
        cursor: pointer;

        .more-txt {
            width: 100%;
            height: 40px;
            line-height: 40px;
            font-size: 14px;
            color: #333333;
            border: 1px solid #eee;
            border-radius: 5px;
            display: inline-block;
            text-align: center;
        }
    }
}
</style>
