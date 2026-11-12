<template>
    <div class="shop-box" v-if="Object.keys(shopInfo).length > 0">
        <div class="shop-info">
            <div class="shop-img">
                <el-image class="" lazy :src="imageFormat(shopInfo.shopLogo)" />
            </div>
            <div class="shop-text-box">
                <div class="shop-title">{{ shopInfo.shopTitle }}</div>
                <div class="shop-desc-text">{{ $t("上架商品:") }} {{ shopInfo.listingCount }}</div>
                <div class="shop-desc-text">{{ $t("店铺收藏数:") }} {{ shopInfo.collectCount }}</div>
                <div class="btn-box flex justify-between">
                    <NuxtLink :to="urlFormat({ path: 'shop', id: shopInfo?.shopId })" class="mr10">
                        <div class="btn">
                            <span class="iconfont-pc icon-guanli"></span>
                            {{ $t("进店逛逛") }}
                        </div>
                    </NuxtLink>
                    <div class="btn collect" @click="addShopCollect(shopInfo?.shopId)">
                        <span class="iconfont-pc"
                            :class="{ 'icon-shoucang': !userShopInfo?.collectShop, 'icon-yishoucang': userShopInfo?.collectShop }">
                        </span>
                        <span>{{ $t("收藏店铺") }}</span>
                    </div>
                </div>
            </div>
        </div>
        <div class="shop-product">
            <div class="product-list">
                <template v-for="(item, index) in shopInfo.listingProduct" :key="item.productId">
                    <div class="product-item">
                        <el-image class="rank-icon" lazy :src="imageList[index]" />
                        <div class="product-img">
                            <NuxtLink :to="urlFormat({ path: 'product', id: item.productId, sn: item.productSn })"
                                target="_blank">
                                <el-image
                                    style="width: 100%; height: 100%;"
                                    lazy
                                    :src="productImage(item)"
                                    fit="cover"
                                    :alt="item.productName || ''"
                                />
                                <img
                                    v-if="styleMap[item.productId]?.borderImage"
                                    class="product-style-border"
                                    :src="styleMap[item.productId].borderImage"
                                />
                            </NuxtLink>
                        </div>
                        <div class="product-text">{{ item.productName }}</div>
                        <FormatPrice class="product-price" v-model="item.productPrice"></FormatPrice>
                    </div>
                </template>
            </div>
        </div>
    </div>
</template>

<script lang="ts" setup>
import { computed, watch } from "vue";
import { useProductStyle } from "~/hooks";
import { getShopList } from "~/api/shop/shop";
import type { shopListItem } from "~/types/shop/shop";
import { useUserStore } from "~/store/user";
import { getShopCollect, getShopDetail } from "~/api/product/product";
import { imageFormat } from "@/utils/format";
import shopTop1 from "~/assets/images/search/shop_top1.png";
import shopTop2 from "~/assets/images/search/shop_top2.png";
import shopTop3 from "~/assets/images/search/shop_top3.png";
import shopTop4 from "~/assets/images/search/shop_top4.png";
import shopTop5 from "~/assets/images/search/shop_top5.png";

const { t } = useI18n();
const props = defineProps({
    keyword: {
        type: String,
        default: ""
    }
});

const imageList = [shopTop1, shopTop2, shopTop3, shopTop4, shopTop5];

const shopInfo = ref<shopListItem>({} as shopListItem);

const listingProducts = computed(() => shopInfo.value?.listingProduct ?? []);
const { styleMap } = useProductStyle(listingProducts);

const getShopData = async () => {
    try {
        const result = await getShopList({ keyword: props.keyword, size: 1 });
        console.log(result, 'resultresult');
        if(result.records.length === 0) {
            shopInfo.value = {} as shopListItem;
            return;
        }
        shopInfo.value = result.records[0];
        await getShop(shopInfo.value.shopId);
    } catch (error) {
        console.error(error);
    }
};

watch(
    () => props.keyword,
    (val) => {
        if (val) {
            getShopData();
        }
    },
    {
        immediate: true
    }
);
const userStore = useUserStore();
const userShopInfo = ref();
const addShopCollect = async (shopId: number) => {
    try {
        await getShopCollect(shopId);
        await getShop(shopId);
        message.success(t("操作成功"));
    } catch (error: any) {
        message.error(error.message);
    } finally {
    }
};

const getShop = async (id: number) => {
    try {
        const result = await getShopDetail(id);
        userShopInfo.value = result;
    } catch (error: any) {
        message.error(error.message);
    } finally {
    }
};
</script>

<style lang="less" scoped>
.shop-box {
    height: 180px;
    width: 100%;
    background: url(/assets/images/search/shop_bg.png) no-repeat #eeeef1;
    margin-top: 10px;
    padding-left: 20px;
    display: flex;

    .shop-info {
        display: flex;
        height: 100%;
        align-items: center;
        width: 30%;

        .shop-img {
            width: 130px;
            height: 130px;
            margin-right: 15px;
            display: flex;
            align-items: center;
        }

        .shop-text-box {
            height: 100%;
            display: flex;
            flex-direction: column;
            justify-content: center;

            .shop-title {
                height: 20px;
                line-height: 20px;
                color: #333;
                white-space: nowrap;
                text-overflow: ellipsis;
                overflow: hidden;
                font:
                    700 14px / 18px "Helvetica Neue",
                    "Hiragino Sans GB",
                    SimSun,
                    serif;
            }

            .shop-desc-text {
                margin-top: 5px;
                white-space: nowrap;
                text-overflow: ellipsis;
                overflow: hidden;
                color: #666;
            }
        }
    }

    .shop-product {
        flex: 1;

        .product-list {
            height: 100%;
            display: flex;
            align-items: center;
            justify-content: space-around;
        }

        .product-item {
            width: 150px;
            height: 160px;
            border-radius: 3px;
            overflow: hidden;
            border-width: 0px;
            border-style: solid;
            background-color: rgb(255, 255, 255);
            cursor: pointer;
            display: flex;
            flex-direction: column;
            align-items: center;
            position: relative;

            .rank-icon {
                position: absolute;
                width: 30px;
                height: 30px;
                top: 0;
                left: 0;
                z-index: 9;
            }

            .product-img {
                width: 105px;
                height: 105px;
                padding-top: 10px;
                position: relative;

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

            .product-text {
                width: 100%;
                height: 20px;
                line-height: 20px;
                text-align: center;
                overflow: hidden;
                white-space: nowrap;
                text-overflow: ellipsis;
                background-color: var(--main-bg);
                color: var(--main-text);
                font-family:
                    Microsoft Yahei,
                    Tahoma,
                    Arial,
                    Roboto,
                    Droid Sans,
                    Helvetica Neue,
                    Droid Sans Fallback,
                    Heiti SC,
                    sans-self;
            }

            .product-price {
                color: var(--general);
                font-weight: bold;
                font-size: 16px;
                margin-top: 6px;
            }
        }
    }
}

.btn-box {
    padding: 15px 0 10px;

    .btn {
        width: 85px;
        height: 30px;
        text-align: center;
        background: #f7f7f7;
        border: 1px solid #ddd;
        cursor: pointer;
        display: flex;
        align-items: center;
        justify-content: center;
        margin-right: 10px;

        &:last-child {
            margin-right: 0;
        }

        &.collect {
            background: #f7f7f7;
            border: 1px solid #ddd;
        }

        .iconfont-pc {
            font-size: 18px;
            margin-right: 5px;
        }

        .icon-guanli,
        .icon-yishoucang {
            color: var(--general);
        }

        .icon-shoucang {
            color: #ddd;
        }
    }
}
</style>
