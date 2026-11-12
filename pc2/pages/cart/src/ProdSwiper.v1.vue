<template>
    <div class="container">
        <div class="sale_prod_list">
            <div class="hot_sale_tit flex align-center justify-center">
                <i class="iconfont-pc icon-aixin"></i>
                <p>{{ $t("猜您喜欢") }}</p>
            </div>
            <div class="hot_sale_tips">{{ $t("实时推荐最适合您的商品") }}</div>
            <div class="cheap_prod">
                <swiper v-if="productList.length > 0" v-model:swiperOption="swiperOption" :itemList="productList">
                    <template v-slot:default="{ item }">
                        <div class="tab-list flex-wrap" style="background-color: #fff">
                            <div v-for="product in item" class="item" to="">
                                <NuxtLink :to="urlFormat({ path: 'product', id: product.productId, sn: product.productSn })" target="_blank">
                                    <img :src="productImage(product)" :alt="product.productName || ''" />
                                </NuxtLink>
                                <p>
                                    <FormatPrice v-model="product.price"></FormatPrice>
                                </p>
                                <p class="ellipsis">
                                    <NuxtLink :to="urlFormat({ path: 'product', id: product.productId, sn: product.productSn })" target="_blank">
                                        {{ product.productName }}
                                    </NuxtLink>
                                </p>
                                <div class="recom_add">
                                    <NuxtLink
                                        :to="urlFormat({ path: 'product', id: product.productId, sn: product.productSn })"
                                        v-if="product.productSku && product.productSku.length > 0"
                                    >
                                        <i class="iconfont-pc icon-shiwu-gouwuche ico-style"></i>
                                    </NuxtLink>
                                    <ProductBuy :shop-id="product.shopId" :id="product.productId" :number="1" @callback="onBuy" v-else>
                                        <i class="iconfont-pc icon-shiwu-gouwuche"></i>
                                    </ProductBuy>
                                </div>
                            </div>
                        </div>
                    </template>
                </swiper>
                <div v-else class="no-data">{{ $t("暂无推荐") }}~</div>
            </div>
        </div>
    </div>
</template>
<script lang="ts" setup>
import { onMounted, reactive, ref } from "vue";
import type { ProductFilterParams, ProductItem } from "~/types/product/product.d";
import { getProductLikeList } from "~/api/product/product";
import { urlFormat } from "~/utils/format";

const swiperOption = ref<any>({
    autoplay: false,
    navigation: true,
    effect: "fade",
    loop: true
});
const emit = defineEmits(["onBuy"]);
// 基本参数定义
const filterState = ref<ProductItem[]>([]);
const loading = ref<boolean>(true);
//显示数量需定义，全局默认为10，参考默认显示3页，即为page*3
const filterParams = reactive<ProductFilterParams>({
    //初使化用于查询的参数
    page: 1,
    size: 30
});
const onBuy = () => {
    emit("onBuy");
};
const loadGuessLike = async () => {
    loading.value = true;
    try {
        const result = await getProductLikeList({ ...filterParams });
        filterState.value = result;
        // 数据处理为swiper格式
        if (filterState.value.length > 0) filterStateToList();
    } catch (error: any) {
        message.error(error.message);
    } finally {
        loading.value = false;
    }
};
onMounted(() => {
    loadGuessLike();
});

const productList = reactive<ProductItem[][]>([]);
const filterStateToList = () => {
    let result = [];
    for (let i = 0; i < filterState.value.length; i += 10) {
        let chunk: any = filterState.value.slice(i, i + 10);
        result.push(chunk);
    }
    productList.push(...result);
};
</script>
<style lang="less" scoped>
.cheap_prod :deep(.swiper) {
    padding-bottom: 30px;
}

.cheap_prod :deep(.swiper-button-prev),
.cheap_prod :deep(.swiper-button-next) {
    z-index: 2;
    border: none;
    outline: none;
    transition: background-color 0.2s ease;
}

.cheap_prod :deep(.swiper-button-prev) {
    left: 10px;
    border-top-right-radius: 2px;
    border-bottom-right-radius: 2px;
}

.cheap_prod :deep(.swiper-button-next) {
    right: 10px;
    border-top-left-radius: 2px;
    border-bottom-left-radius: 2px;
}

.cheap_prod :deep(.swiper-button-prev:after),
.cheap_prod :deep(.swiper-button-next:after) {
    color: #e5e5e5;
    font-size: 24px;
    font-weight: bolder;
}

.cheap_prod :deep(.swiper-pagination) {
    padding-left: 25px;
    padding-bottom: 10px;
    text-align: center;
}

.cheap_prod :deep(.swiper-pagination-bullet) {
    position: relative;
    display: inline-block;
    transition: background 0.2s ease;
    width: 12px;
    height: 12px;
    margin-right: 10px;
    background: #c0c0c0;
    border-radius: 100px;
}

.cheap_prod :deep(.swiper-pagination-bullet-active) {
    width: 20px;
    height: 12px;
    background-color: var(--general);
    left: 0;
}

.sale_prod_list {
    margin-top: 50px;

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

    .cheap_prod {
        overflow: hidden;
        border: 1px solid #f0f0f0;
        border-radius: 2px;
        background: #fff;
        padding: 40px 0 20px 0;

        .no-data {
            text-align: center;
            font-size: 20px;
            color: #999999;
        }

        .tab-list {
            font-size: 13px;
            margin: 0 80px;

            .item {
                cursor: pointer;
                width: 205px;
                margin-bottom: 25px;
                text-align: center;
                position: relative;

                img {
                    width: 150px;
                    height: 150px;
                }

                p {
                    width: 150px;
                    text-align: center;
                    margin: 5px auto 0;
                }

                .recom_add {
                    background-color: var(--general);
                    opacity: 0.8;
                    width: 30px;
                    height: 30px;
                    color: var(--main-text);
                    font-size: 20px;
                    line-height: 30px;
                    font-weight: 700;
                    border-radius: 100px;
                    position: absolute;
                    right: 40px;
                    top: 110px;
                    z-index: 99;
                    display: none;
                    .ico-style {
                        color: var(--main-text);
                        background-color: var(--general);
                    }
                }

                &:hover {
                    .recom_add {
                        display: block;
                    }
                }
            }
        }
    }
}
</style>
