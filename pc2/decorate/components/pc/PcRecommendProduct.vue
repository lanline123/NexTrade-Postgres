<template>
    <div class="dec-recommend-product" :style="cssVars">
        <h3 v-if="titleName" class="dec-recommend-product__title">{{ titleName }}</h3>
        <div class="dec-recommend-product__list">
            <div v-for="item in productList" :key="item.productId" class="dec-recommend-product__item">
                <div class="dec-recommend-product__card">
                    <div class="dec-recommend-product__flip">
                        <div class="dec-recommend-product__pic">
                            <NuxtLink
                                class="dec-recommend-product__pic-link"
                                :to="urlFormat({ path: 'product', seoLink: item.seoLink, id: item.productId, sn: item.productSn })"
                                target="_blank"
                            >
                                <el-image :src="productImage(item)" lazy :alt="item.productName || ''" />
                                <img
                                    v-if="styleMap[item.productId]?.borderImage"
                                    class="dec-recommend-product__border"
                                    :src="styleMap[item.productId].borderImage || undefined"
                                    :alt="item.productName || ''"
                                />
                            </NuxtLink>
                        </div>
                        <div class="dec-recommend-product__name">
                            <NuxtLink :to="urlFormat({ path: 'product', seoLink: item.seoLink, id: item.productId, sn: item.productSn })" target="_blank">
                                <span v-if="item.deliveryMode === 2 && item.deliveryType === 1" class="dec-recommend-product__tag">{{ $t("周期购") }}</span>
                                <span v-if="item.deliveryMode === 2 && item.deliveryType === 2" class="dec-recommend-product__tag">{{ $t("次卡兑换") }}</span>
                                {{ item.productName }}
                            </NuxtLink>
                        </div>
                        <FormatPrice v-model="item.productPrice" class="dec-recommend-product__price"></FormatPrice>
                    </div>
                    <div class="dec-recommend-product__operate">
                        <NuxtLink
                            :to="urlFormat({ path: 'product', seoLink: item.seoLink, id: item.productId, sn: item.productSn })"
                            class="dec-recommend-product__detail"
                            target="_blank"
                        >
                            {{ $t("查看详情") }}
                        </NuxtLink>
                        <template v-if="item.productSku && item.productSku.length > 0">
                            <NuxtLink :to="urlFormat({ path: 'product', seoLink: item.seoLink, id: item.productId, sn: item.productSn })" target="_blank">
                                <div class="dec-recommend-product__buy">{{ $t(getBuyText(item.productType)) }}</div>
                            </NuxtLink>
                        </template>
                        <template v-else>
                            <ProductBuy
                                :shop-id="item.shopId"
                                :isQuick="item.productType !== 1"
                                :id="item.productId"
                                :disabled="item.productStock == 0"
                                :number="1"
                            >
                                <span class="dec-recommend-product__buy">{{ $t(getBuyText(item.productType)) }}</span>
                            </ProductBuy>
                        </template>
                    </div>
                </div>
            </div>
        </div>
        <div ref="recommendMoreRef">
            <div v-if="isLoading" class="dec-recommend-product__loading">
                <div class="gif-loading"></div>
            </div>
            <div v-if="isNoMore && productList.length > 0" class="dec-recommend-product__end">{{ $t("没有更多喽") }}~</div>
        </div>
    </div>
</template>

<script lang="ts" setup>
/**
 * PC 推荐商品瀑布流（新套 kebab 模块 pc-recommend-product）
 * 由旧套 pcRecommendProduct 迁移而来：滚动分页取数不变，列数与间距改由 frame 驱动。
 */
import { computed, onMounted, onUnmounted, ref, type PropType } from "vue";
import { productImage, urlFormat } from "~/utils/format";
import { getProductsList } from "@/api/common";
import { useProductStyle } from "~/hooks";
import type { ProductsListResult } from "@/types/common/common";
import type { DecorateModulePayload } from "~/types/decoration/decorateModule";

const props = defineProps({
    module: { type: Object as PropType<DecorateModulePayload>, default: () => ({}) }
});

const PAGE_SIZE = 20;

const moduleConfig = computed<Record<string, any>>(() => (props.module || {}) as Record<string, any>);
const frame = computed<Record<string, any>>(() => moduleConfig.value.frame || {});
const products = computed<Record<string, any>>(() => moduleConfig.value.products || {});
const titleName = computed(() => moduleConfig.value.titleName || "");

const cssVars = computed(() => {
    const f = frame.value;
    const columns = Number(f.columns) > 0 ? Number(f.columns) : 5;
    return {
        "--dec-box-padding": `${Number(f.boxPadding) || 0}px`,
        "--dec-box-padding-top": `${Number(f.boxPaddingTop) || 0}px`,
        "--dec-box-padding-bottom": `${Number(f.boxPaddingBottom) || 0}px`,
        "--dec-box-radius": `${Number(f.boxRadius) || 0}px`,
        "--dec-bg-color": f.backgroundColor || "transparent",
        "--dec-item-gap": `${Number(f.itemGap) >= 0 ? Number(f.itemGap) : 16}px`,
        "--dec-columns": String(columns)
    };
});

const paramsValueMap: { [key: number]: string[] } = {
    1: ["productIds"],
    2: ["productCategoryId"],
    3: ["productTag"]
};
const paramsKeyMap: { [key: string]: string } = {
    productIds: "ids",
    productCategoryId: "categoryId",
    productTag: "introType"
};

const isLoading = ref(true);
const isNoMore = ref(false);
const page = ref(1);
const productList = ref<ProductsListResult[]>([]);
const { styleMap } = useProductStyle(productList);

const maxNum = computed(() => {
    const selectType = Number(products.value.productSelectType);
    if (selectType === 1) {
        const ids = products.value.productIds;
        return Array.isArray(ids) ? ids.length : 0;
    }
    return Number(products.value.productNumber) || 0;
});

const size = computed(() => {
    return PAGE_SIZE * page.value < maxNum.value ? PAGE_SIZE : maxNum.value - PAGE_SIZE * (page.value - 1);
});

const params = computed<Record<string, unknown>>(() => {
    const param: Record<string, unknown> = {};
    const selectType = Number(products.value.productSelectType);
    const keys = paramsValueMap[selectType];
    if (!keys) return param;

    if (selectType === 1) {
        const ids = products.value.productIds;
        param[paramsKeyMap.productIds!] = Array.isArray(ids) ? ids.join(",") : String(ids ?? "");
    } else {
        param.page = 1;
        keys.forEach((key) => {
            const apiKey = paramsKeyMap[key];
            if (apiKey) param[apiKey] = products.value[key];
        });
    }
    return param;
});

const getProductsListData = async () => {
    if (size.value <= 0) {
        isLoading.value = false;
        isNoMore.value = true;
        return;
    }
    isLoading.value = true;
    try {
        const result = await getProductsList({ ...params.value, page: page.value, size: size.value });
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
    if (scrollY + windowHeight > containerTop + containerHeight && productList.value.length < maxNum.value && !isLoading.value && !isNoMore.value) {
        page.value++;
        getProductsListData();
    }
};

onMounted(() => {
    window.addEventListener("scroll", handleScroll);
    getProductsListData();
});

onUnmounted(() => {
    window.removeEventListener("scroll", handleScroll);
});
</script>

<style lang="less" scoped>
.dec-recommend-product {
    box-sizing: border-box;
    width: 100%;
    padding-left: var(--dec-box-padding, 0);
    padding-right: var(--dec-box-padding, 0);
    padding-top: var(--dec-box-padding-top, 0);
    padding-bottom: var(--dec-box-padding-bottom, 0);
    border-radius: var(--dec-box-radius, 0);
    background-color: var(--dec-bg-color, transparent);

    &__title {
        position: relative;
        height: 45px;
        margin: 20px auto;
        font-size: 28px;
        font-weight: 700;
        line-height: 45px;
        text-align: center;
        color: #333;
    }

    &__list {
        display: grid;
        grid-template-columns: repeat(var(--dec-columns, 5), minmax(0, 1fr));
        gap: var(--dec-item-gap, 16px);
        width: 100%;
    }

    &__item {
        min-width: 0;
    }

    &__card {
        display: block;
        box-sizing: border-box;
        height: 299px;
        padding: 8px;
        overflow: hidden;
        background-color: #fff;
        color: #333;

        &:hover {
            .dec-recommend-product__name {
                height: 0;
                opacity: 0;
            }

            .dec-recommend-product__operate {
                display: flex;
            }
        }
    }

    &__pic {
        margin: 0 auto;
        padding: 30px;
        text-align: center;
    }

    &__pic-link {
        position: relative;
        display: inline-block;
        width: 160px;
        height: 160px;
        vertical-align: top;

        :deep(img) {
            width: 160px;
            height: 160px;
        }
    }

    &__border {
        position: absolute;
        left: 0;
        top: 0;
        width: 100%;
        height: 100%;
        z-index: 5;
        pointer-events: none;
        object-fit: fill;
    }

    &__name {
        position: relative;
        height: 40px;
        margin-top: 5px;
        padding: 0 12px;
        overflow: hidden;
        text-align: left;
        text-overflow: ellipsis;
        word-break: break-all;
        display: -webkit-box;
        -webkit-line-clamp: 2;
        -webkit-box-orient: vertical;
        font-size: 14px;
        line-height: 20px;
        color: #666;
        transition: all 0.5s ease;
    }

    &__tag {
        position: relative;
        margin-right: 8px;
        color: var(--general);

        &::after {
            content: "";
            position: absolute;
            right: -5px;
            top: 50%;
            transform: translateY(-45%);
            width: 1px;
            height: 70%;
            background-color: var(--general);
        }
    }

    &__price {
        display: flex;
        align-items: center;
        justify-content: center;
        height: 30px;
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

    &__operate {
        display: none;
        margin-top: 2px;
        text-align: center;
        line-height: 40px;
    }

    &__detail {
        width: 100px;
        height: 40px;
        margin-right: 10px;
        background-color: var(--vice-bg);
        color: var(--vice-text);
        cursor: pointer;
    }

    &__buy {
        width: 100px;
        height: 40px;
        background-color: var(--general);
        color: var(--main-text);
        cursor: pointer;
    }

    &__loading {
        display: flex;
        justify-content: center;
    }

    &__end {
        margin-bottom: 20px;
        text-align: center;
    }
}
</style>
