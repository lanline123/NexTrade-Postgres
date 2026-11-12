<template>
    <div class="goods-list__box" ref="productBox">
        <div class="goods-list-c">
            <div class="wrap">
                <div class="item" v-for="(item, index) in productList" :key="index">
                    <div class="g-item-warp">
                        <div class="pic">
                            <NuxtLink :to="urlFormat(getTargetPath(item))" target="_blank">
                                <el-image class="ease" lazy fit="contain" :src="productImage(item)" :alt="item.productName || ''" />
                            </NuxtLink>
                            <img v-if="styleMap[item.productId]?.borderImage" class="product-style-border" :src="styleMap[item.productId].borderImage" />
                        </div>
                        <div class="info">
                            <FormatPrice class="" v-model="item.price" :fontStyle="{ fontSize: '24px', fontWeight: 'bold' }" :currencyStyle="{ fontSize: '14px', fontWeight: 'bold' }" currencyAlignment="bottom"></FormatPrice>
                            <FormatPrice v-if="item.marketPrice && Number(item.marketPrice) > 0 && Number(item.marketPrice) > Number(item.price)" class="market-price" v-model="item.marketPrice" :fontStyle="{ fontSize: '14px', color: '#999', textDecoration: 'line-through' }" :currencyStyle="{ fontSize: '12px', color: '#999', textDecoration: 'line-through' }" currencyAlignment="bottom"></FormatPrice>
                            <div class="name">
                                <NuxtLink :to="urlFormat({ path: 'product', seoLink: item.seoLink, id: item.productId, sn: item.productSn })" target="_blank">
                                    <span v-if="item.productSaleMode" class="presale-text">{{ $t("预售") }}</span>
                                    <span v-if="item.productSaleMode">|</span>
                                    {{ item.productName }}
                                </NuxtLink>
                            </div>
                            <template v-if="isMerchant()">
                                <div class="shop-info">
                                    <NuxtLink class="shop-name" v-if="item.shop" :to="urlFormat({ path: 'shop', id: item.shopId })" target="_blank">{{
                                        item.shop.shopTitle
                                    }}</NuxtLink>
                                    <div class="shop-name" v-else>
                                        {{ commonStore.defaultShopName ? commonStore.defaultShopName : $t("官方自营") }}
                                    </div>
                                </div>
                            </template>
                            <!-- <div class="brief">
                                <div class="desc" v-if="item.productBrief">{{ item.productBrief }}</div>
                            </div> -->

                            <div class="promotion-box">
                                <template
                                    v-if="
                                        promotionList[item.productId] &&
                                        promotionList[item.productId].activityInfo &&
                                        promotionList[item.productId].activityInfo.length > 0
                                    "
                                >
                                    <div class="promotion-box-item" v-for="subItem in promotionList[item.productId].activityInfo" :key="subItem.productId">
                                        <div class="activity" v-if="subItem.type != 2 && getActivityText(subItem.type)">
                                            {{ $t(getActivityText(subItem.type)) }}
                                        </div>
                                        <div class="promotion" v-if="subItem.type == 2">
                                            <div class="ticket-type">{{ $t("券") }} <span class="ticket-line"></span></div>
                                            <!-- <div class="ticket-type">{{ $t("券") }} <span class="ticket-line"></span></div> -->
                                            <div class="ticket-content">{{ subItem.data.promotionDesc }}</div>
                                        </div>
                                    </div>
                                </template>
                            </div>

                            <div class="item_cart_num">
                                <div class="shopping_num">
                                    <input v-model="item.number" type="text" @input="validateInput($event, item)" />
                                    <span>
                                        <a class="add" :class="numAddDisabled(item)" href="javascript:;" @click="increment(item)"></a>
                                        <a
                                            class="decrease"
                                            :class="item.number && item.number <= 1 ? 'disabled' : ''"
                                            href="javascript:;"
                                            @click="decrement(item)"
                                        ></a>
                                    </span>
                                </div>
                                <div class="shopping_act">
                                    <NuxtLink
                                        :to="urlFormat({ path: 'product', seoLink: item.seoLink, id: item.productId, sn: item.productSn })"
                                        v-if="item.productSku && item.productSku.length > 0 && !item.productSaleMode"
                                    >
                                        <el-button size="small" type="primary">{{ $t(getBuyText(item.productType)) }}</el-button>
                                    </NuxtLink>
                                    <!-- 预售商品：有sku时跳转到详情 -->
                                    <NuxtLink
                                        v-else-if="item.productSaleMode && item.productSku && item.productSku.length > 0"
                                        :to="urlFormat({ path: 'product', seoLink: item.seoLink, id: item.productId, sn: item.productSn })"
                                        target="_blank"
                                    >
                                        <el-button size="small" type="primary">{{ $t("立即购买") }}</el-button>
                                    </NuxtLink>
                                    <!-- 预售商品：没有sku时只显示立即购买按钮 -->
                                    <ProductBuy
                                        v-else-if="item.productSaleMode && !item.productSku"
                                        :isQuick="true"
                                        :id="item.productId"
                                        :disabled="item.productStock == 0 || item.seckillStock == 0"
                                        :number="item.number"
                                    >
                                        <el-button :disabled="item.productStock == 0 || item.seckillStock == 0" size="small" type="primary">{{
                                            $t("立即购买")
                                        }}</el-button>
                                    </ProductBuy>

                                    <!-- 没有SKU且不是预售商品：根据productType判断 -->
                                    <ProductBuy
                                        v-else
                                        :isQuick="item.productType !== 1"
                                        :id="item.productId"
                                        :disabled="item.productStock == 0 || item.seckillStock == 0"
                                        :number="item.number"
                                    >
                                        <el-button :disabled="item.productStock == 0 || item.seckillStock == 0" size="small" type="primary">{{
                                            $t(getBuyText(item.productType))
                                        }}</el-button>
                                    </ProductBuy>
                                </div>
                            </div>
                        </div>
                    </div>
                </div>
                <div class="search-empty" v-if="productList.length === 0 && !loadingProduct">
                    <div class="bd">
                        <h4>{{ $t("好狠的筛选条件啊，把相关的商品都埋没了！") }}</h4>
                        <p>{{ $t("建议您") }}</p>
                        <p>{{ $t("1、适当减少筛选条件") }}</p>
                        <p>{{ $t("2、调整价格区间") }}</p>
                        <p>{{ $t("3、尝试其他关键字") }}</p>
                        <el-button class="graybtn mt10" @click="goBack">{{ $t("返回上一步") }}</el-button>
                    </div>
                </div>
            </div>
        </div>
    </div>
    <div class="pagination">
        <Pagination v-if="searchStore.total > 0" v-model:page="filter.page" :size="size" :total="searchStore.total" @callback="pageChange" />
    </div>
</template>
<script lang="ts" setup>
import { ref } from "vue";
import { useCommonStore } from "@/store/common";
import { getSearchProductList } from "@/api/search/search";
import { getPromotion } from "@/api/product/product";
import { urlFormat, productImage } from "@/utils/format";
import { useSearchStore } from "@/store/search";
import { Pagination } from "@/components/list";
import type { SearchFilter } from "@/types/search/search.d";
import type { ProductItem, PromotionList, PromotionFilterParams } from "@/types/product/product.d";
import { isMerchant } from "@/utils/util";
import getBuyText from "@/utils/getBuyText";
import { useProductStyle } from "~/hooks";
const { t } = useI18n();
const commonStore = useCommonStore();

const props = defineProps({
    queryParams: {
        type: Object,
        default: {}
    },
    attrs: {
        type: Array,
        default: () => []
    }
});
const searchStore = useSearchStore();
const size = ref(25);
const emit = defineEmits(["change", "pageChange", "filterUndo"]);
const filter = defineModel<SearchFilter>("filter") as Ref<SearchFilter>;
const queryParams = defineModel("queryParams");
const loadingProduct = defineModel("loadingProduct");
const productList = ref<ProductItem[]>([]);
const { styleMap } = useProductStyle(productList);
const promotionList = ref<{ [key: string]: PromotionList }>({} as { [key: string]: PromotionList });

const getSearchProduct = async () => {
    try {
        loadingProduct.value = true;
        productList.value = [];
        const result = await getSearchProductList({ ...props.queryParams, size: size.value, attrs: props.attrs, isDelete: 0 });
        productList.value = result.records;
        searchStore.total = result.total ?? 0;
        productList.value.forEach((item) => {
            item.number = 1;
        });
        const productIds = result.records.map((item: any) => ({ productId: item.productId }));
        getPromotionList({ products: productIds, from: "list" });
    } catch (error: any) {
        console.error(error);
        searchStore.total = 0;
        message.error(error?.message || t("商品加载失败，请稍后重试"));
    } finally {
        loadingProduct.value = false;
    }
};

const getPromotionList = async (data: PromotionFilterParams) => {
    try {
        const result = await getPromotion({
            products: data.products,
            shopId: data.shopId,
            from: data.from
        });
        promotionList.value = result;
    } catch (error) {
        console.error(error);
    }
};
//1秒杀2优惠券3满减4折扣5满赠6限时折扣
type ActivityInfo = {
    [key: number]: string;
};
const activitType: ActivityInfo = {
    1: "秒杀价",
    2: "券",
    3: "满减",
    4: "折扣",
    5: "满赠",
    6: "限时折扣",
    7: "拼团"
};
const getActivityText = (type: number) => {
    if (activitType[type]) {
        return activitType[type];
    }
    return "";
};

watch(
    () => [queryParams.value, props.attrs],
    () => {
        if (loadingProduct.value) return;
        getSearchProduct();
    },
    { deep: true, immediate: true }
);
const productBox: Ref<HTMLElement | null> = ref(null);
const pageChange = () => {
    emit("change", filter.value.page);
    emit("pageChange");
};

const validateInput = (event: any, item: ProductItem) => {
    // 使用正则表达式只允许数字输入
    const numericValue = event?.target.value.replace(/[^0-9]/g, ""); // 替换非数字字符
    if (numericValue == "") {
        item.number = 1;
        return;
    }
    item.number = numericValue ? Math.max(1, Number(numericValue)) : undefined;
};

// 禁用添加数量的判断
const numAddDisabled = (item: ProductItem) => {
    if (item.limitNumber > 0 && item.limitNumber <= item.number) {
        return "disabled";
    } else if (item.productStock > 0 && item.productStock <= item.number) {
        return "disabled";
    }
};

// 加减
const increment = (item: ProductItem) => {
    if (item.limitNumber > 0 && item.limitNumber <= item.number) {
        message.error(t(`限购${item.limitNumber}件，已达限购数`));
        return;
    } else if (item.productStock > 0 && item.productStock <= item.number) {
        message.error(t("库存不足"));
        return;
    }
    if (item.number) item.number++;
};
const decrement = (item: ProductItem) => {
    if (item.number && item.number > 1) {
        item.number--;
    }
};
const goBack = () => {
    emit("filterUndo");
};
const getTargetPath = (item: ProductItem) => {
    if (promotionList.value[`${item.productId}`]?.activityInfo[0]?.type === 7) {
        return { path: "product", seoLink: item.seoLink, id: item.productId, sn: item.productSn };
    }
    return { path: "product", seoLink: item.seoLink, id: item.productId, sn: item.productSn };
};
</script>
<style lang="less" scoped>
.promotion-box {
    display: flex;
    align-items: center;
    height: 20px;
    padding-top: 5px;

    .activity {
        max-width: 100px;
        background-color: var(--general);
        color: var(--main-text);
        padding: 0 8px;
        border-radius: 2px;
        padding-bottom: 2px;
        margin-right: 10px;
    }

    .promotion {
        display: flex;
        background-color: var(--tag-bg);
        color: var(--tag-text);
        border-radius: 2px;
        padding-bottom: 2px;

        .ticket-type {
            padding: 0 6px;
            max-width: 50px;
            position: relative;

            .ticket-line {
                position: absolute;
                display: inline-block;
                height: 100%;
                opacity: 0.9;
                width: 2px;
                top: 0;
                right: 0px;
                transform: scaleY(0.7);
                background: linear-gradient(
                    to bottom,
                    var(--ump-border, #c9c9ff) 20%,
                    var(--ump-border, #c9c9ff) 20%,
                    var(--ump-border, #c9c9ff) 20%,
                    rgba(255, 255, 255, 0) 20%,
                    rgba(255, 255, 255, 0) 35%,
                    rgba(136, 76, 255, 1) 35%,
                    var(--ump-border, #c9c9ff) 35%,
                    var(--ump-border, #c9c9ff) 35%,
                    var(--ump-border, #c9c9ff) 60%,
                    rgba(255, 255, 255, 0) 60%,
                    rgba(255, 255, 255, 0) 55%,
                    rgba(255, 255, 255, 0) 75%,
                    var(--ump-border, #c9c9ff) 75%,
                    var(--ump-border, #c9c9ff) 75%,
                    var(--ump-border, #c9c9ff) 100%
                );
            }
        }

        .ticket-content {
            padding: 0 4px;
            max-width: 90px;
            overflow: hidden;
            text-overflow: ellipsis;
            white-space: nowrap;
        }
    }
}

.shop-info {
    line-height: 18px;
    height: 18px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    cursor: pointer;

    .shop-name {
        color: #50607a;
    }
}

.goods-list-c {
    width: 100%;
    overflow: hidden;

    .wrap {
        display: flex;
        flex-wrap: wrap;
        min-height: 400px;

        .item {
            border: 1px solid #fff;
            padding: 18px 9px 20px;
            transition: border-color 0.1s ease 0s;
            margin: 0 5px 14px 0;
            position: relative;
            width: calc((100% - 25px) / 6);
            box-sizing: border-box;
            background: #fff;

            .pic {
                position: relative;
                width: 100%;
                height: auto;
                aspect-ratio: 1;
                margin-bottom: 10px;
                display: flex;
                align-items: center;
                justify-content: center;
                overflow: hidden;

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

                a {
                    height: 100%;
                    width: 100%;
                    display: flex;
                    align-items: center;
                    justify-content: center;
                }

                .ease {
                    width: 100%;
                    height: 100%;
                    object-fit: contain;
                    object-position: center;
                }
            }

            .info {
                :deep(.market-price) {
                    margin-left: 6px;
                    line-height: 27px;
                }

                :deep(.price) {
                    height: 27px;
                    line-height: 26px;
                    color: var(--price);

                    .num {
                        font-size: 24px;
                        font-weight: normal;
                    }

                    .util {
                        font-size: 12px;
                        margin-bottom: -2px;
                    }
                }

                .name {
                    color: #11192d;
                    font-size: 16px;
                    font-weight: 500;
                    height: 48px;
                    letter-spacing: 0;
                    line-height: 24px;
                    overflow: hidden;
                    width: 100%;
                    word-break: break-all;
                    display: -webkit-box;
                    -webkit-line-clamp: 2;
                    -webkit-box-orient: vertical;

                    a {
                        color: #11192d;
                        color: #11192d;
                        font-size: 16px;
                        font-weight: 500;
                        height: 48px;
                        letter-spacing: 0;
                        line-height: 24px;
                        overflow: hidden;
                        width: 100%;
                        word-break: break-all;

                        .presale-text {
                            color: var(--general);
                            margin-right: 4px;
                            font-weight: 500;
                        }
                    }
                }

                .brief {
                    height: 20px;
                    line-height: 20px;
                    overflow: hidden;

                    .desc {
                        color: #ff4040;
                    }
                }
            }
        }

        .item:nth-child(6n) {
            margin-right: 0;
        }

        @media (max-width: 1649px) {
            .item {
                width: calc((100% - 20px) / 5);
            }

            .item:nth-child(6n) {
                margin-right: 5px;
            }

            .item:nth-child(5n) {
                margin-right: 0;
            }
        }

        @media (max-width: 1379px) {
            .item {
                width: calc((100% - 15px) / 4);
            }

            .item:nth-child(5n),
            .item:nth-child(6n) {
                margin-right: 5px;
            }

            .item:nth-child(4n) {
                margin-right: 0;
            }
        }

        .item:hover {
            border-color: #e9e9e9;
            box-shadow: 0 0 2px 2px #f8f8f8;
        }
    }
}

.item_cart_num {
    display: flex;
    align-items: center;
    padding-top: 10px;

    .shopping_num {
        display: flex;
        align-items: center;
        margin-right: 8px;

        input {
            background: url("/assets/images/category/search.png") no-repeat scroll 0px -310px;
            background-position: 0 -310px;
            border: 0 none;
            color: #555;
            padding: 0;
            height: 28px;
            line-height: 28px;
            text-align: center;
            width: 30px;
        }

        span {
            height: 28px;
            margin-left: 2px;
            width: 16px;
        }

        a {
            background: url("/assets/images/category/search.png") no-repeat scroll;
            cursor: pointer;
            display: block;
            height: 14px;
            line-height: 14px;
            overflow: hidden;
            text-indent: -99em;
            width: 16px;
        }

        .add {
            background-position: -17px -281px;
        }

        .decrease {
            background-position: -17px -295px;
        }

        .decrease.disabled {
            background-position: 0 -295px;
        }
        .add.disabled {
            background-position: 0 -281px;
        }
    }

    .shopping_act {
        height: 28px;
        overflow: hidden;
        display: flex;
        align-items: center;
    }
}

.search-empty {
    margin-bottom: 10px;
    position: relative;
    display: flex;
    justify-content: center;
    width: 100%;
}

.search-empty .bd {
    background: url("/assets/images/common/empty.gif") no-repeat scroll 56px 45px;
    padding: 45px 50px 45px 200px;
}

.search-empty h4 {
    font-size: 14px;
    padding-bottom: 10px;
}

.search-empty p {
    color: #666666;
    margin-bottom: 5px;
}

.pagination {
    display: flex;
    justify-content: end;
    margin-top: 10px;
    margin-bottom: 20px;
}
</style>
