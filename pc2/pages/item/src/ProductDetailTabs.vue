<!--
  商品详情 · 老版 Tab 区（产品详情 / 用户评价 / 商品咨询 / 售后服务）
  - 仅老版 UI 使用；与新版 ProductDetailNew 对称
  - Tab 切换与滚动定位内聚在本组件
-->
<template>
    <div class="lyecs_main">
        <div data-nav-rev="azIyODJlcHg" class="goods_tab_wrap sticky-element">
            <div class="tabs">
                <ul class="flex">
                    <li @click="tabChange(1)">
                        <a href="javascript:;" tab-data="info" class="tab_a" :class="tabIndex == 1 ? 'current' : ''"
                            ><span>{{ $t("产品详情") }}</span></a
                        >
                    </li>

                    <!-- D1146：评价功能关闭时不展示「用户评价」Tab -->
                    <li @click="tabChange(2)" v-if="commentEnabled">
                        <a href="javascript:;" tab-data="com" class="tab_a" :class="tabIndex == 2 ? 'current' : ''"
                            ><span
                                >{{ $t("用户评价") }}<em>({{ rankDetail.total }})</em></span
                            ></a
                        >
                    </li>
                    <li @click="tabChange(3)">
                        <a href="javascript:;" tab-data="ask" class="tab_a" :class="tabIndex == 3 ? 'current' : ''"
                            ><span
                                >{{ $t("商品咨询") }}<em>({{ consultationTotal }})</em></span
                            ></a
                        >
                    </li>
                    <li @click="tabChange(4)">
                        <a href="javascript:;" tab-data="ser" class="tab_a" :class="tabIndex == 4 ? 'current' : ''"
                            ><span>{{ $t("售后服务") }}</span></a
                        >
                    </li>
                </ul>
            </div>
        </div>
        <div class="goods_desc paidcontent" v-if="tabIndex == 1" id="xiangqing">
            <template v-if="isPro() && product.productType === 4">
                <div class="goods_description">
                    <template v-if="product.isBuy && product.paidContent">
                        <template v-for="item in product.paidContent" :key="item">
                            <img class="el-img" v-if="item.type == 'pic'" fit="cover" :src="imageFormat(item.pic || '')" :alt="product.productName || ''" />
                            <ClientOnly>
                                <div class="desc-text-item" v-if="item.type == 'text'" v-html="formatRichText(item.html)"></div>
                            </ClientOnly>
                            <div class="desc-video-item" v-if="item.type == 'video' && item.videoUrl">
                                <video
                                    :src="imageFormat(item.videoUrl)"
                                    :poster="item.cover ? imageFormat(item.cover) : undefined"
                                    controls
                                    preload="metadata"
                                    style="width: 100%; display: block"
                                ></video>
                            </div>
                        </template>
                    </template>
                    <template v-else>
                        <img fit="cover" class="img" src="~/assets/images/product/paidcontent-bg.png" />
                    </template>
                </div>
            </template>
            <template v-if="attrList.normal && attrList.normal.length > 0">
                <ul class="shuxin flex">
                    <template v-for="(item, index) in attrList.normal" :key="index">
                        <li>
                            {{ item.attrName }}：
                            <template v-for="(attr, arrtIndex) in item.attrList" :key="arrtIndex"> {{ attr.attrValue }} &nbsp; </template>
                        </li>
                    </template>
                </ul>
            </template>

            <div class="goods_description">
                <template v-for="item in descArr">
                    <img class="el-img" v-if="item.type == 'pic'" fit="cover" :src="imageFormat(item.pic || '')" :alt="product.productName || ''" />
                    <ClientOnly>
                        <div class="desc-text-item" v-if="item.type == 'text'" v-html="formatRichText(item.html)"></div>
                    </ClientOnly>
                    <div class="desc-video-item" v-if="item.type == 'video' && item.videoUrl">
                        <video
                            :src="imageFormat(item.videoUrl)"
                            :poster="item.cover ? imageFormat(item.cover) : undefined"
                            controls
                            preload="metadata"
                            style="width: 100%; display: block"
                        ></video>
                    </div>
                </template>
            </div>
        </div>

        <div class="goods_desc shouhou" v-show="tabIndex == 4" id="shouhou">
            <div class="tab_title">
                <h2><span class="icon"></span>{{ $t("售后服务") }}</h2>
            </div>
            <ClientOnly>
                <ProductAfterSaleService></ProductAfterSaleService>
            </ClientOnly>
        </div>

        <div class="goods_desc" v-if="tabIndex == 2 && commentEnabled" id="pingjia">
            <div class="tab_title">
                <h2><span class="icon"></span>{{ $t("用户评价") }}</h2>
            </div>
            <ProductComment v-model="product.productId"></ProductComment>
        </div>
        <div class="goods_desc goods_ask" v-if="tabIndex == 3" id="zixun">
            <div class="tab_title">
                <h2><span class="icon"></span>{{ $t("商品咨询") }}</h2>
            </div>
            <ProductConsultation :product="product" :productStock="productStock" :productNumber="productNumber"> </ProductConsultation>
        </div>
    </div>
</template>

<script lang="ts" setup>
import { computed, ref, nextTick } from "vue";
import type { PropType } from "vue";
import { formatRichText } from "@/utils/util";
import { useCommonStore } from "~/store/common";
import type { AttrList, DescArr, ProductItem, RankDetail } from "~/types/product/product.d";

const commonStore = useCommonStore();
// D1146：商品评价总开关
const commentEnabled = computed(() => commonStore.productCommentEnabled === 1);

defineProps({
    product: {
        type: Object as PropType<ProductItem>,
        required: true
    },
    attrList: {
        type: Object as PropType<AttrList>,
        default: () => ({ normal: [] })
    },
    descArr: {
        type: Array as PropType<DescArr[]>,
        default: () => []
    },
    rankDetail: {
        type: Object as PropType<RankDetail>,
        default: () => ({})
    },
    consultationTotal: {
        type: Number,
        default: 0
    },
    productStock: {
        type: Number,
        default: 0
    },
    productNumber: {
        type: Number,
        default: 1
    }
});

const tabIndex = ref(1);

/**
 * @description 切换商品详情页Tab（详情/评价/咨询/售后）并滚动到内容区
 * @param data Tab索引值
 */
const tabChange = async (data: any) => {
    tabIndex.value = data;
    await nextTick();
    const tabBarElement: any = document.querySelector(".lyecs_main");
    tabBarElement?.scrollIntoView({ behavior: "smooth", block: "start" });
};
</script>

<style lang="less" scoped>
.lyecs_main {
    width: 964px;
    margin-left: 10px;
    position: relative;

    .goods_tab_wrap {
        background-color: #f6f6f6;
        border: 1px solid #eee;
        border-bottom: none;

        .tabs {
            .tab_a {
                display: inline-block;
                padding: 0 20px;
                font-size: 14px;
                text-align: center;
                text-decoration: none;
                line-height: 36px;
                height: 36px;
            }

            .current {
                height: 34px;
                background-color: #fff;
                border-bottom: 1px solid #fff;
                border-top: 3px solid var(--general);
                line-height: 32px;
                margin-top: -1px;
                color: var(--general);
            }
        }
    }

    .goods_desc {
        padding: 10px;
        border: 1px solid #eee;
        margin-bottom: 10px;

        .shuxin {
            background: none repeat scroll 0 0 #f9f9f9;
            border: 1px solid #e8e8e8;
            padding: 10px;
            height: 100%;
            margin-bottom: 10px;
            flex-wrap: wrap;

            li {
                line-height: 26px;
                overflow: hidden;
                width: 300px;
                font-weight: normal;
                padding: 0 10px;
                box-sizing: border-box;
            }
        }

        .goods_description {
            .el-img {
                width: 100%;
                display: block;
            }
        }

        .tab_title {
            background: #f6f6f6;
            height: 30px;
            padding-left: 10px;
            margin-bottom: 10px;
            padding-top: 7px;

            h2 {
                display: inline-block;
                font-family: "微软雅黑", "SimHei";
                font-size: 16px;
                font-weight: normal;
            }

            .icon {
                background: url("/assets/images/product/product_title_bg.png") no-repeat scroll -80px 0;
                display: inline-block;
                height: 16px;
                line-height: 16px;
                margin-right: 5px;
                margin-top: 4px;
                vertical-align: top;
                width: 16px;
            }
        }
    }

    .shouhou {
        .tab_title {
            .icon {
                background-position: -112px 0;
            }
        }
    }

    .goods_ask {
        .tab_title {
            .icon {
                background-position: -128px 0;
            }
        }
    }
}

.paidcontent {
    .img {
        width: 100%;
    }
}
</style>
