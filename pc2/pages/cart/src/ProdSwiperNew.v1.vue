<template>
    <div class="container">
        <div class="mod_you_like">
            <div class="hot_sale_tit flex align-center justify-center">
                <i class="iconfont-pc icon-aixin"></i>
                <p>{{ $t("猜您喜欢") }}</p>
            </div>
            <div class="hot_sale_tips">{{ $t("实时推荐最适合您的商品") }}</div>
            <div class="you_like_list">
                <ul>
                    <template v-for="(item, index) in productList" :key="index">
                        <li>
                            <div class="item">
                                <div class="flip_wrap">
                                    <div class="pro_pic">
                                        <NuxtLink
                                            :to="urlFormat({ path: 'product', id: item.productId, sn: item.productSn })"
                                            target="_blank">
                                            <el-image :src="productImage(item)" lazy :alt="item.productName || ''" />
                                        </NuxtLink>
                                    </div>
                                    <div class="pro_name">
                                        <!-- <el-tag color="var(--general)" plain>{{$t('拼团')}}</el-tag> -->
                                        <NuxtLink
                                            :to="urlFormat({ path: 'product', id: item.productId, sn: item.productSn })"
                                            target="_blank">{{ item.productName }}
                                        </NuxtLink>
                                    </div>
                                    <FormatPrice v-model="item.productPrice" class="pro_price"></FormatPrice>
                                </div>
                                <div class="operate_area">
                                    <NuxtLink
                                        :to="urlFormat({ path: 'product', id: item.productId, sn: item.productSn })"
                                        class="not_interest left-span">
                                        <NuxtLink
                                            :to="urlFormat({ path: 'product', id: item.productId, sn: item.productSn })"
                                            target="_blank">{{
                                                $t("查看详情")
                                            }}</NuxtLink>
                                    </NuxtLink>
                                    <template v-if="item.productSku && item.productSku.length > 0">
                                        <NuxtLink
                                            :to="urlFormat({ path: 'product', id: item.productId, sn: item.productSn })"
                                            target="_blank">
                                            <div class="view_detail">{{ $t(getBuyText(item.productType)) }}</div>
                                        </NuxtLink>
                                    </template>
                                    <template v-else>
                                        <ProductBuy :shop-id="item.shopId" :isQuick="item.productType !== 1" :id="item.productId"
                                            :disabled="item.productStock == 0" :number="1">
                                            <span class="view_detail">{{ $t(getBuyText(item.productType)) }}</span>
                                        </ProductBuy>
                                    </template>
                                </div>
                            </div>
                        </li>
                    </template>
                </ul>
            </div>
            <div ref="recommendMoreRef">
                <template v-if="isLoading">
                    <div class="loading-con">
                        <div class="gif-loading"></div>
                    </div>
                </template>
                <template v-if="isloadEnd" class="load-end-con">
                    <div class="">{{ $t("没有更多喽") }}~</div>
                </template>
            </div>
        </div>
    </div>
</template>
<script lang="ts" setup>
import { onMounted, onUnmounted, ref } from "vue";
import { productImage, urlFormat } from "@/utils/format";
import { getProductsList } from "@/api/common";
import type { ProductsListResult } from "@/types/common/common";
import { getProductLikeListIds } from "~/api/product/product";

const isLoading = ref(true);
const isloadEnd = ref(false);
const page = ref(1);
const total = ref(0);
const productList = ref<ProductsListResult[]>([]);

const size = computed(() => {
    let num = 10;
    return num * page.value < total.value ? num : total.value - num * (page.value - 1);
});

const isNoMore = ref(false);
const getProductsListData = async () => {
    isLoading.value = true;
    try {
        const ids = await getProductLikeListIds();
        const result = await getProductsList({ page: page.value, size: 10, ids });
        total.value = result.total;
        productList.value = [...productList.value, ...result.records];
        if (result.records.length < size.value) {
            isNoMore.value = true;
        }
    } catch (error) {
        console.log(error);
    } finally {
        isLoading.value = false;
    }
};

const recommendMoreRef = ref<HTMLElement | null>(null);

const handleScroll = () => {
    const container = recommendMoreRef.value;
    if (!container) return;

    const containerTop = container.offsetTop;
    const containerHeight = container.offsetHeight;
    const windowHeight = window.innerHeight;
    const scrollY = window.scrollY;
    // 检查是否滚动到指定容器
    if (scrollY + windowHeight > containerTop + containerHeight && productList.value.length < total.value && !isLoading.value && !isNoMore.value) {
        page.value++;
        getProductsListData();
    }
};

// 监听滚动事件
onMounted(() => {
    window.addEventListener("scroll", handleScroll);
    getProductsListData();
});

// 移除滚动事件监听
onUnmounted(() => {
    window.removeEventListener("scroll", handleScroll);
});
</script>
<style lang="less" scoped>
.loading-con {
    display: flex;
    justify-content: center;
}

.load-end-con {
    text-align: center;
    margin-bottom: 20px;
}

.com-pc-title {
    position: relative;
    width: 150px;
    height: 45px;
    font-size: 28px;
    font-weight: 700;
    text-align: center;
    line-height: 45px;
    padding: 0 30px;
    margin: 20px auto 20px;
    overflow: hidden;
    color: #333;
    display: flex;
    justify-content: space-around;
}

.loading-con {
    display: flex;
    justify-content: center;
}

.load-end-con {
    text-align: center;
    margin-bottom: 20px;
}

.com-pc-title:after,
.com-pc-title:before {
    content: "";
    position: absolute;
    top: 50%;
    margin-top: -10px;
}

.com-pc-title:after,
.com-pc-title:before {
    background-size: 50px 20px;
}

.com-pc-title:after,
.com-pc-title:before {
    background-image: url(/assets/images/common/sprite.png);
    width: 25px;
    height: 20px;
}

.com-pc-title:before {
    background-position: 0 0;
    left: 0;
}

.com-pc-title:after {
    background-position: -25px 0;
    right: 0;
}

.com-pc-title:after,
.com-pc-title:before {
    background-image: url(/assets/images/common/sprite@2x.png);
}

.hot_sale_tit {
    height: 40px;
    color: var(--general);
    line-height: 40px;
    font-size: 16px;
    font-family: "微软雅黑";

    i {
        margin-right: 5px;
        display: inline-block;
    }
}

.hot_sale_tips {
    padding-bottom: 12px;
    color: #999;
    text-align: center;
    line-height: 12px;
}

.mod_you_like {
    margin-top: 30px;
    width: 100%;
}

.mod_you_like .you_like_list {
    position: relative;
    padding-bottom: 50px;
    overflow: hidden;
}

.mod_you_like .you_like_list ul {
    margin-right: -12px;
    overflow: hidden;
    display: flex;
    flex-wrap: wrap;
}

.mod_you_like .you_like_list li {
    width: 228px;
    margin: 12px 12px 0 0;
}

.mod_you_like .you_like_list li .item {
    display: block;
    width: 212px;
    height: 299px;
    overflow: hidden;
    margin: 0 auto;
    padding: 8px;
    background-color: #fff;
    color: #333;
}

.mod_you_like .you_like_list li .item:hover {
    text-decoration: none;
}

.mod_you_like .you_like_list li .item:hover .pro_name {
    height: 0;
    opacity: 0;
}

.mod_you_like .you_like_list li .item:hover .operate_area {
    display: flex;
}

.mod_you_like .you_like_list .flip_wrap {
    margin: 0 auto;
}

.mod_you_like .you_like_list .pro_pic {
    margin: 0 auto;
    text-align: center;
    padding: 30px;
}

.mod_you_like .you_like_list .pro_pic :deep(img) {
    width: 160px;
    height: 160px;
}

.mod_you_like .you_like_list .pro_name {
    position: relative;
    height: 40px;
    overflow: hidden;
    margin-top: 5px;
    line-height: 20px;
    text-align: left;
    -webkit-transition: all 0.5s ease;
    -o-transition: all 0.5s ease;
    -moz-transition: all 0.5s ease;
    transition: all 0.5s ease;
    font:
        12px/1.5 Microsoft YaHei,
        Heiti SC,
        tahoma,
        arial,
        Hiragino Sans GB,
        "\5B8B\4F53",
        sans-serif;
    color: #666;
    font-size: 14px;
    padding: 0 12px;
    word-break: break-all;
    overflow: hidden;
    text-overflow: ellipsis;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    -webkit-box-orient: vertical;
}

.mod_you_like .you_like_list .pro_price {
    height: 30px;
    align-items: center;
    justify-content: center;
    display: flex;
    line-height: 30px;
    color: var(--general);
    font-weight: bold;

    :deep(.num) {
        font-weight: bold;
        font-size: 20px;
        font-family: arial, sans-serif;
    }

    :deep(.util) {
        margin-bottom: -5px;
    }

    :deep(.decimals) {
        font-size: 12px;
        font-weight: bold;
    }
}

.mod_you_like .you_like_list .operate_area {
    display: none;
    margin-top: 2px;
    text-align: center;
    line-height: 40px;
}

.mod_you_like .you_like_list .operate_area span {
    width: 100px;
    height: 40px;
}

.mod_you_like .you_like_list .operate_area .left-span {
    width: 100px;
    height: 40px;
}

.mod_you_like .you_like_list .operate_area .not_interest {
    margin-right: 10px;
    background-color: var(--vice-bg);
    color: var(--vice-text);
    cursor: pointer;

    &>a {
        color: var(--vice-text);
    }
}

.mod_you_like .you_like_list .operate_area .view_detail {
    background-color: var(--general);
    width: 100px;
    height: 40px;
    color: var(--main-text);
    cursor: pointer;
}
</style>
