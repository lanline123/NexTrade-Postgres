<template>
    <div class="competitors-row hand-pointer">
        <div class="tit flex justify-between align-center" v-if="type !== 'rank'">
            <div class="name">{{ title }}</div>
            <NuxtLink class="txt ellipsis" target="_blank" v-if="isMore" :to="`/search/?intro=best`">
                <a class="more">{{ $t("更多") }}</a>
            </NuxtLink>
        </div>
        <div class="tit flex justify-between align-center" v-if="type == 'rank' && checkEmpty(cateRanke)">
            <div class="name">{{ title }}</div>
            <NuxtLink class="txt ellipsis" target="_blank" v-if="isMore" :to="`/search/?intro=best`">
                <a class="more">{{ $t("更多") }}</a>
            </NuxtLink>
        </div>
        <div class="col-text flex-wrap justify-between" v-if="type == 'cate'">
            <NuxtLink class="txt ellipsis" target="_blank" v-for="item in list" :to="urlFormat({ path: 'list', seoLink: item.seoLink, id: item.categoryId })">{{
                item.categoryName
            }}</NuxtLink>
        </div>
        <div class="col-text flex-wrap justify-between" v-if="type == 'brand'">
            <NuxtLink class="txt ellipsis" target="_blank" v-for="item in list" :to="urlFormat({ path: 'brand', id: item.brandId })">{{
                item.brandName
            }}</NuxtLink>
        </div>
        <div class="col-ranking" v-if="type == 'rank' && checkEmpty(cateRanke)">
            <div class="ranking-tab flex">
                <div class="tab" v-for="(item, index) in tabList" :class="activeKey == item.type ? 'active' : ''" @mouseover="handleMouseOver(item.type)">
                    {{ $t(item.label) }}
                </div>
            </div>
            <div class="ranking-list" v-if="cateRanke[activeKey].length > 0">
                <div class="item flex align-start" v-for="(item, index) in cateRanke[activeKey]">
                    <div class="num">{{ index + 1 }}</div>
                    <NuxtLink target="_blank" :to="urlFormat({ path: 'product', seoLink: item.seoLink, sn: item.productSn })">
                        <div class="product-info flex align-start">
                            <img :src="productImage(item)" :alt="item.productName || ''" />
                            <div class="info">
                                <a class="ellipsis2">{{ item.productName }}</a>
                                <template v-if="item.price && Number(item.price) > 0">
                                    <div class="money">
                                        <FormatPrice v-model="item.price"></FormatPrice>
                                    </div>
                                </template>
                            </div>
                        </div>
                    </NuxtLink>
                </div>
            </div>
            <div class="default" v-else>{{ $t("暂无数据") }}</div>
        </div>
        <div class="col-product" v-if="type == 'lookAlso'">
            <div class="item" v-for="item in list">
                <NuxtLink target="_blank" :to="urlFormat({ path: 'product', seoLink: item.seoLink, sn: item.productSn })">
                    <img :src="productImage(item)" :alt="item.productName || ''" />

                    <template v-if="item.price && Number(item.price) > 0">
                        <div class="money">
                            <FormatPrice v-model="item.price"></FormatPrice>
                        </div>
                    </template>

                    <a class="name ellipsis2">{{ item.productName }}</a>
                </NuxtLink>
            </div>
        </div>
        <div class="col-article" v-if="type == 'article'">
            <div class="item" v-for="item in list">
                <NuxtLink target="_blank" :to="`/article/news/list?articleId=${item.articleId}`">
                    <a class="ellipsis2">{{ item.articleTitle }}</a>
                </NuxtLink>
            </div>
        </div>
        <div class="col-product" v-if="type == 'relatedList'">
            <div class="item" v-for="item in list">
                <NuxtLink target="_blank" :to="urlFormat({ path: 'product', seoLink: item.seoLink, sn: item.productSn })">
                    <img :src="productImage(item)" :alt="item.productName || ''" />
                    <template v-if="item.price && Number(item.price) > 0">
                        <div class="money">
                            <FormatPrice v-model="item.price"></FormatPrice>
                        </div>
                    </template>
                    <a class="name ellipsis2">{{ item.productName }}</a>
                </NuxtLink>
            </div>
        </div>
    </div>
</template>

<script setup>
import { reactive, ref } from "vue";
import { productImage, urlFormat } from "@/utils/format";
const props = defineProps({
    title: {
        type: String,
        default: ""
    },
    type: {
        type: String,
        default: ""
    },
    isMore: {
        type: Boolean,
        default: false
    },
    list: {
        type: Array,
        default: () => []
    },
    cateRanke: {
        type: Object,
        default: {}
    }
});
const tabList = reactive([
    {
        label: "同价位",
        type: "price"
    },
    {
        label: "同品牌",
        type: "brand"
    },
    {
        label: "同类别",
        type: "cate"
    }
]);
const checkEmpty = (obj) => {
    for (const key in obj) {
        if (obj[key].length > 0) {
            return true;
        }
    }
    return false;
};
const activeKey = ref("price");

const handleMouseOver = (data) => {
    activeKey.value = data;
};
</script>
<style lang="less" scoped>
.competitors-row {
    width: 216px;
    border: 1px solid #eeeeee;
    margin-bottom: 10px;
    .tit {
        border-bottom: 0;
        background: #f6f6f6;
        padding: 8px 12px;
        line-height: 25px;
        height: 25px;

        .name {
            font-size: 16px;
        }
    }

    .col-text {
        padding: 15px;
        .txt {
            display: block;
            width: 80px;
            height: 30px;
            line-height: 30px;
            padding: 0 5px;
        }
        &::after {
            content: "";
            display: block;
            width: 80px;
            height: 0px;
        }
    }
    .col-ranking {
        padding: 10px 10px 0 10px;
        .ranking-tab {
            background: #f3f3f3;
            height: 30px;
            line-height: 30px;
            .tab {
                width: 64px;
                text-align: center;
                border-bottom: 1px solid #ccc;
            }
            .active {
                background-color: #fff;
                border: 1px solid #ccc;
                color: var(--general);
                font-weight: 600;
                border-bottom: none;
            }
        }
        .ranking-list {
            .item {
                border-top: 1px solid #eee;
                padding: 10px 0;
                &:first-child {
                    border: none;
                }
                .num {
                    background: #fff;
                    border: 1px solid #ddd;
                    color: var(--general);
                    font-size: 10px;
                    height: 12px;
                    line-height: 12px;
                    text-align: center;
                    width: 12px;
                    margin-right: 10px;
                }
                .product-info {
                    img {
                        width: 50px;
                        height: 50px;
                        margin-right: 10px;
                    }
                    .info {
                        width: 96px;
                        a {
                            width: 100%;
                        }
                        .money {
                            color: var(--price);
                        }
                    }
                }
            }
        }
        .default {
            text-align: center;
            padding: 30px 0;
        }
    }
    .col-product {
        padding: 0 10px;
        .item {
            display: flex;
            flex-direction: column;
            align-items: center;
            padding: 15px 0;
            text-align: center;
            img {
                width: 160px;
                height: 160px;
            }
            .money {
                width: 100%;
                margin-top: 7px;
                color: var(--price);
                margin-bottom: 5px;
                text-align: left;
                :deep(.price) {
                    .util {
                        font-size: 18px;
                    }
                    font-size: 18px;
                }
            }
            .name {
                text-align: left;
                color: #666;
            }
        }
    }
    .col-article {
        padding: 10px 10px;
        .item {
            margin: 10px 0;
            a {
                display: block;
                background: url(/assets/images/icon.gif) no-repeat 0 -24px;
                line-height: 17px;
                overflow: hidden;
                height: 16px;
                text-indent: 10px;
            }
        }
    }
}
</style>
