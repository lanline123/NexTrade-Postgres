<template>
    <div
        class="goods-list__box"
        ref="productBox"
        :class="{
            'goods-list__box--loading': loadingProduct,
            'goods-list__box--empty': productList.length === 0 && !loadingProduct,
            'goods-list__box--new-ui': isNewUi
        }"
    >
        <div class="goods-list-c">
            <div class="wrap" ref="wrapRef">
                <div
                    class="item"
                    v-for="(item, index) in productList"
                    :key="index"
                    @mouseenter="onNewUiItemEnter"
                    @mouseleave="onNewUiItemLeave"
                >
                    <div class="g-item-warp">
                        <div class="pic">
                            <NuxtLink :to="cardProductLink(item, getTargetPath(item))" target="_blank">
                                <el-image
                                    v-sold-out="{
                                        productStock: item.productStock,
                                        productStatus: item.productStatus,
                                        productType: item.productType,
                                        deliveryMode: item.deliveryMode,
                                        deliveryType: item.deliveryType
                                    }"
                                    fit="contain"
                                    class="ease"
                                    lazy
                                    :src="productImage(item)"
                                    :alt="item.productName || ''"
                                />
                            </NuxtLink>
                            <img
                                v-if="styleMap[item.productId]?.borderImage"
                                class="product-style-border"
                                :src="styleMap[item.productId]?.borderImage || ''"
                            />
                        </div>
                        <div class="info">
                            <FormatPrice class="" :model-value="cardPrice(item, (item as any).price)"></FormatPrice>
                            <div class="name">
                                <span v-if="item.deliveryMode === 2 && item.deliveryType === 1" class="cycleBuyTag">{{ $t("周期购") }}</span>
                                <span v-if="item.deliveryMode === 2 && item.deliveryType === 2" class="cycleBuyTag">{{ $t("次卡兑换") }}</span>
                                <NuxtLink
                                    :to="cardProductLink(item, { path: 'product', seoLink: item.seoLink, id: item.productId, sn: item.productSn })"
                                    target="_blank"
                                >
                                    <span v-if="item.productSaleMode" class="presale-text">{{ $t("预售") }}</span>
                                    <span v-if="item.productSaleMode">|</span>
                                    {{ item.productName }}
                                </NuxtLink>
                            </div>
                            <div class="shop-info">
                                <NuxtLink class="shop-name" v-if="item.shop"
                                    :to="{ path: '/store/info', query: { shopId: item.shopId } }" target="_blank">{{
                                        item.shop.shopTitle
                                    }}</NuxtLink>
                                <div class="shop-name" v-else>
                                    {{ commonStore.defaultShopName ? commonStore.defaultShopName : $t("官方自营") }}
                                </div>
                            </div>
                            <!-- <div class="brief">
                                <div class="desc" v-if="item.productBrief">{{ item.productBrief }}</div>
                            </div> -->

                            <div class="promotion-box">
                                <template v-if="
                                    promotionList[item.productId] &&
                                    promotionList[item.productId].activityInfo &&
                                    promotionList[item.productId].activityInfo.length > 0
                                ">
                                    <div class="promotion-box-item"
                                        v-for="subItem in promotionList[item.productId].activityInfo"
                                        :key="subItem.productId">
                                        <div class="activity" v-if="subItem.type != 2 && getActivityText(subItem.type)">
                                            {{ $t(getActivityText(subItem.type)) }}
                                        </div>
                                        <div class="promotion" v-if="subItem.type == 2">
                                            <div class="ticket-type">{{ $t("券") }} <span class="ticket-line"></span>
                                            </div>
                                            <div class="ticket-content">{{ subItem.data.promotionDesc }}</div>
                                        </div>
                                    </div>
                                </template>
                            </div>

                            <div class="item_cart_num">
                                <div class="shopping_num">
                                    <input v-model="item.number" type="text"
                                        @input="validateInput($event, item)" />
                                    <span>
                                        <a class="add" :class="numAddDisabled(item)" href="javascript:;" @click="increment(item)"></a>
                                        <a class="decrease" :class="item.number && item.number <= 1 ? 'disabled' : ''"
                                            href="javascript:;" @click="decrement(item)"></a>
                                    </span>
                                </div>
                                <div v-if="item.deliveryMode === 2" class="shopping_act">
                                    <el-button size="small" type="primary" @click="handleToMobileProduct(item)">{{
                                        $t(getBuyText(item.productType === 1 ? 0 : item.productType))
                                    }}</el-button>
                                </div>
                                <div class="shopping_act" v-else>
                                    <NuxtLink
                                        :to="cardProductLink(item, { path: 'product', seoLink: item.seoLink, id: item.productId, sn: item.productSn })"
                                        v-if="item.productSku && item.productSku.length > 1">
                                        <el-button size="small" type="primary">{{ $t(item.productSaleMode ? "立即购买" : getBuyText(item.productType)) }}</el-button>
                                    </NuxtLink>
                                    <!-- 预售商品：无SKU时可直接购买 -->
                                    <ProductBuy
                                        v-else-if="item.productSaleMode"
                                        :shop-id="item.shopId"
                                        :isQuick="true"
                                        :id="item.productId"
                                        :disabled="item.productStock == 0 || item.seckillStock == 0"
                                        :number="item.number"
                                    >
                                        <el-button :disabled="item.productStock == 0 || item.seckillStock == 0" size="small" type="primary">{{ $t("立即购买") }}</el-button>
                                    </ProductBuy>
                                    <ProductBuy
                                        v-else
                                        :shop-id="item.shopId"
                                        :isQuick="item.productType !== 1"
                                        :id="item.productId"
                                        :disabled="item.productStock == 0 || item.seckillStock == 0"
                                        :number="item.number"
                                    >
                                        <el-button :disabled="item.productStock == 0 || item.seckillStock == 0" size="small" type="primary">{{ $t(getBuyText(item.productType)) }}</el-button>
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
        <div v-if="loadingProduct" class="goods-list-loading">
            <div class="goods-list-loading__spinner"></div>
        </div>
    </div>
    <div class="pagination">
        <Pagination v-if="searchStore.total > 0" v-model:page="filter.page" :size="size" :total="searchStore.total"
            @callback="pageChange" />
    </div>
</template>
<script lang="ts" setup>
import { ref, nextTick, onMounted, onBeforeUnmount } from "vue";
import { useCommonStore } from "@/store/common";
import { getSearchProductList } from "@/api/search/search";
import { getPromotion } from "@/api/product/product";
import { urlFormat, productImage } from "@/utils/format";
import { useSearchStore } from "@/store/search";
import { Pagination } from "@/components/list";
import { useRouter } from "vue-router";
import type { SearchFilter } from "@/types/search/search.d";
import type { ProductItem, PromotionList, PromotionFilterParams } from "@/types/product/product.d";
import getBuyText from "@/utils/getBuyText";
import { useProductStyle } from "~/hooks";
import { message } from "ant-design-vue";
import { useProductCardBuy } from "@/hooks/useProductCardBuy";

const { t } = useI18n();
const commonStore = useCommonStore();
const { cardPrice, cardProductLink } = useProductCardBuy();

const handleToMobileProduct = (item: any) => {
    let url = `/pcMobile?path=/pages/product/index&id=${item.productId}`;
    if (item.shopId) {
        url += `&shopId=${item.shopId}`;
    }
    window.open(url, "_blank");
};

const props = defineProps({
    queryParams: {
        type: Object,
        default: {}
    },
    attrs: {
        type: Array,
        default: () => []
    },
    isNewUi: {
        type: Boolean,
        default: false
    }
});
const searchStore = useSearchStore();
const router = useRouter();
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
    } catch (error) {
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
    if(!activitType[type]) {
        return '';
    }
    // 兜底：后端若返回未定义的活动类型，避免 $t(undefined) 导致报错
    return activitType[type] ?? "";
};

watch(
    () => queryParams.value,
    () => {
        getSearchProduct();
    },
    { deep: true, immediate: true }
);
watch(
    () => props.attrs,
    () => {
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
    if(numericValue === '') {
        item.number = 1;
        return;
    }
    // 如果输入的数字小于1，则设置为1
    item.number = numericValue ? Math.max(1, Number(numericValue)) : undefined;
};

// 禁用添加数量的判断
const numAddDisabled = (item: ProductItem) => {
    if(item.limitNumber > 0 && item.limitNumber <= item.number) {
        return 'disabled';
    } else if(item.productStock > 0 && item.productStock <= item.number) {
        return 'disabled';
    }
};

// 加减
const increment = (item: ProductItem) => {
    if(item.limitNumber > 0 && item.limitNumber <= item.number) {
        message.error(t("限购{0}件，已达限购数", [item.limitNumber]));
        return;
    } else if(item.productStock > 0 && item.productStock <= item.number) {
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
    // 撤销逻辑统一交给上层 useSearchPage.onFilterUndo（逐级撤销筛选项）
    emit("filterUndo");
};
const getTargetPath = (item: ProductItem) => {
    if (promotionList.value[`${item.productId}`]?.activityInfo[0]?.type === 7) {
        return { path: 'product', seoLink: item.seoLink, id: item.productId, sn: item.productSn };
    }
    return { path: 'product', seoLink: item.seoLink, id: item.productId, sn: item.productSn };
};

// ===================== 新版 UI 悬浮卡片 =====================
const wrapRef = ref<HTMLElement | null>(null);

/** 缓存悬浮前 .item 的自然高度，悬浮时用于占位，避免父级坍塌 */
const cacheNewUiItemHeights = () => {
    if (!props.isNewUi || !wrapRef.value) return;
    nextTick(() => {
        wrapRef.value?.querySelectorAll<HTMLElement>(".item").forEach((el) => {
            if (el.matches(":hover")) return;
            el.dataset.baseHeight = String(el.offsetHeight);
        });
    });
};

watch(
    () => [productList.value.length, loadingProduct.value, props.isNewUi] as const,
    () => {
        if (!loadingProduct.value) {
            cacheNewUiItemHeights();
        }
    }
);

const onWindowResize = () => {
    cacheNewUiItemHeights();
};

onMounted(() => {
    cacheNewUiItemHeights();
    if (typeof window !== "undefined") {
        window.addEventListener("resize", onWindowResize);
    }
});

onBeforeUnmount(() => {
    if (typeof window !== "undefined") {
        window.removeEventListener("resize", onWindowResize);
    }
});

/** 新 UI 悬浮时 g-item-warp 会 absolute 脱离文档流，用悬浮前高度占位 */
const onNewUiItemEnter = (event: MouseEvent) => {
    if (!props.isNewUi) return;
    const el = event.currentTarget as HTMLElement;
    const baseHeight = el.dataset.baseHeight;
    if (baseHeight) {
        el.style.minHeight = `${baseHeight}px`;
    }
};

const onNewUiItemLeave = (event: MouseEvent) => {
    if (!props.isNewUi) return;
    const el = event.currentTarget as HTMLElement;
    el.style.minHeight = "";
    requestAnimationFrame(() => {
        el.dataset.baseHeight = String(el.offsetHeight);
    });
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
                background: linear-gradient(to bottom,
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
                        var(--ump-border, #c9c9ff) 100%);
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
        color: #999;
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
            width: 214px;
            background: #fff;

            .pic {
                position: relative;
                width: 214px;
                height: 214px;
                margin-bottom: 10px;

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
                }

                .ease {
                    height: 100%;
                    width: 100%;
                    object-fit: cover;
                }
            }

            .info {
                :deep(.price) {
                    height: 27px;
                    line-height: 27px;
                    color: var(--price);

                    .num {
                        font-size: 18px;
                        font-weight: normal;
                    }

                    .util {
                        font-size: 12px;
                        margin-bottom: -2px;
                    }
                }

                .name {
                    height: 40px;
                    line-height: 20px;
                    word-break: break-all;
                    display: -webkit-box;
                    -webkit-line-clamp: 2;
                    -webkit-box-orient: vertical;
                    overflow: hidden;

                    a {
                        color: #666;
                    }

                    .presale-text {
                        color: var(--general);
                        margin-right: 4px;
                        font-weight: 500;
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

        .item:nth-child(5n) {
            margin-right: 0;
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

.cycleBuyTag {
    color: var(--general);
    position: relative;
    margin-right: 8px;
    &::after {
        content: "";
        position: absolute;
        right: -4.5px;
        top: 50%;
        transform: translateY(-45%);
        width: 1px;
        height: 70%;
        background-color: var(--general);
    }
}

// ===================== 新版 UI / 加载态 =====================
.goods-list__box {
    position: relative;
}

.goods-list__box--empty,
.goods-list__box--loading {
    min-height: 620px;
}

.goods-list-loading {
    position: absolute;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    min-height: 620px;
    background: #fff;
    z-index: 5;
}

.goods-list-loading__spinner {
    width: 40px;
    height: 40px;
    color: var(--general);
    border: 3px solid color-mix(in srgb, var(--general) 15%, transparent);
    border-top-color: var(--general);
    border-radius: 50%;
    animation: goods-list-loading-spin 0.8s linear infinite;
}

@keyframes goods-list-loading-spin {
    from {
        transform: rotate(0deg);
    }

    to {
        transform: rotate(360deg);
    }
}

.goods-list__box--new-ui {
    max-width: 1540px;
    width: 100%;
    margin-left: -8px;
    .goods-list-c {
        overflow: visible;
        .wrap {
            .item {
                position: relative;
                margin: 0;
                padding: 18px 8px 20px;
                width: calc(100% / 6);
                box-sizing: border-box;
                height: fit-content;

                .pic {
                    width: 100%;
                    height: auto;
                    aspect-ratio: 1;
                    display: flex;
                    align-items: center;
                    justify-content: center;
                    overflow: hidden;

                    .ease {
                        width: 100%;
                        height: 100%;
                        object-fit: contain;
                    }
                }

                .g-item-warp {
                    width: 100%;
                    .item_cart_num {
                        display: none;
                    }
                }

                &:hover {
                    height: fit-content;
                    border-radius: 8px;

                    .g-item-warp {
                        position: absolute;
                        width: calc(100% + 12px);
                        padding: 18px 14px 20px;
                        left: 50%;
                        top: 0;
                        transform: translateX(-50%);
                        z-index: 3;
                        background: #fff;
                        border-radius: 16px;
                        box-shadow: rgba(31, 31, 51, 0.12) 0px 12px 36px 0px;
                        box-sizing: border-box;
                        .item_cart_num {
                            display: flex;
                        }
                    }
                }
            }

            .item:nth-child(5n) {
                margin-right: 0;
            }

            @media (max-width: 1649px) {
                .item {
                    width: calc(100% / 5);
                }
            }

            @media (max-width: 1379px) {
                .item {
                    width: calc(100% / 4);
                }
            }
        }
    }
}
</style>
