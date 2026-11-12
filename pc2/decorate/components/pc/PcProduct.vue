<template>
    <TotalContainer v-if="productList?.length > 0" :module="containerModule" class="dec-pc-product" :style="cssVars">
        <!-- 内容容器：左右内边距同时作用于标题与商品列表，保证标题栏宽度与后台预览一致 -->
        <div class="dec-pc-product__content" :style="innerWrapStyle">
            <Title :module="module.title"></Title>
            <template v-if="productList.length > 0">
                <div class="dec-pc-product__wrapper">
                    <div class="dec-pc-product__list">
                        <div v-for="item in productList" :key="item.productId" class="dec-pc-product__item">
                            <NuxtLink target="_blank" :to="urlFormat({ path: 'product', seoLink: item.seoLink, sn: item.productSn })" class="dec-pc-product__card">
                                <div class="dec-pc-product__photo">
                                    <el-image class="dec-pc-product__img" :src="productImage(item)" fit="cover" :alt="item.productName" />
                                </div>
                                <div class="dec-pc-product__info goods-info-ajIyODJjcHc">
                                    <div v-if="showName" class="dec-pc-product__name">
                                        {{ item.productName }}
                                    </div>
                                    <div v-if="showBrief" class="dec-pc-product__brief">
                                        {{ item.productBrief || item.brief }}
                                    </div>
                                    <div class="dec-pc-product__bottom">
                                        <div v-if="showPrice" class="dec-pc-product__price">
                                            <FormatPrice v-model="item.productPrice" :font-style="priceFontStyle" />
                                        </div>
                                        <div v-if="buyBtnConfig.show" class="dec-pc-product__buy" @click.prevent.stop>
                                            <template v-if="isReserveProduct(item) || hasMultiSpecSku(item)">
                                                <div @click.stop="openProduct(item)">
                                                    <ProductButton :buy-btn-style="buyBtnConfig.style"></ProductButton>
                                                </div>
                                            </template>
                                            <template v-else>
                                                <ProductBuy
                                                    :id="resolveProductId(item)"
                                                    :sku-id="resolveSkuId(item)"
                                                    :is-quick="true"
                                                    :disabled="item.productStock == 0"
                                                    :number="1"
                                                >
                                                    <ProductButton :buy-btn-style="buyBtnConfig.style"></ProductButton>
                                                </ProductBuy>
                                            </template>
                                        </div>
                                    </div>
                                </div>
                            </NuxtLink>
                        </div>
                    </div>
                </div>
            </template>
        </div>
    </TotalContainer>
</template>

<script lang="ts" setup>
// 商品列表组件

// ===================== Vue 核心 =====================
import { computed, ref, type PropType } from "vue";

// ===================== 组件导入 =====================
import TotalContainer from "~/decorate/components/shop/TotalContainer.vue";
import Title from "~/decorate/components/shop/Title.vue";
import ProductButton from "~/components/shop/product/ProductButton.vue";

// ===================== API 导入 =====================
import { getProductsList } from "@/api/common";

// ===================== 工具函数 =====================
import { productImage, urlFormat } from "~/utils/format";
import { normalizeBuyBtnStyle } from "~/composables/useBuyBtnStyle";
import type { PcDecorateProductRow } from "~/types/decoration/pcDecorateProduct";
import type { DecorateModulePayload } from "~/types/decoration/decorateModule";
import type { PcSeckillFrameDecor } from "~/types/decoration/pcSeckillModule";

type PcProductModuleConfig = DecorateModulePayload & {
    frame?: PcSeckillFrameDecor;
    products?: Record<string, unknown> & {
        productSelectType?: number;
        productIds?: unknown[];
        [key: string]: unknown;
    };
    style?: number | string;
    goodsPadding?: number;
    title?: unknown;
};

// ===================== Props & Emits =====================
const props = defineProps({
    module: {
        type: Object,
        default: () => ({})
    },
    moduleIndex: {
        type: Number,
        default: 0
    },
    shopId: {
        type: [String, Number] as PropType<string | number>,
        default: -1
    }
});

// ===================== 响应式状态 =====================
const productList = ref<PcDecorateProductRow[]>([]);

const paramsValueMap: { [key: number]: string[] } = {
    1: ["productIds"],
    2: ["productCategoryId", "productNumber"],
    3: ["productTag", "productNumber"]
};

// 取数键映射：NexTrade 后端 product/product/list 用 ids（逗号串）而非 idList
const paramsKeyMap: { [key: string]: string } = {
    productIds: "ids",
    productCategoryId: "categoryId",
    productNumber: "size",
    productTag: "introType"
};

// ===================== 计算属性 =====================
const containerModule = computed(() => {
    // 优先级与后台预览一致：frame -> data(flat) -> default
    const data = (props.module ?? {}) as PcProductModuleConfig;
    const frame = (data.frame ?? {}) as PcSeckillFrameDecor;

    return {
        boxPadding: frame?.boxPadding ?? data?.boxPadding ?? 10,
        boxPaddingTop: frame?.boxPaddingTop ?? data?.boxPaddingTop ?? 5,
        boxPaddingBottom: frame?.boxPaddingBottom ?? data?.boxPaddingBottom ?? 5,
        boxRadius: frame?.boxRadius ?? data?.boxRadius ?? 0,
        backgroundColor: frame?.backgroundColor || data?.backgroundColor || "#ffffff",
        innerPadding: frame?.innerPadding ?? data?.innerPadding ?? 0
    };
});

const innerWrapStyle = computed(() => {
    const data = (props.module ?? {}) as {
        innerPadding?: number;
        frame?: {
            leftAndRightPadding?: number;
            innerPadding?: number;
        };
    };
    const frame = data.frame ?? {};
    const xPadding = frame.leftAndRightPadding ?? frame.innerPadding ?? data.innerPadding ?? 0;
    return {
        paddingLeft: `${xPadding}px`,
        paddingRight: `${xPadding}px`
    };
});

const columns = computed(() => {
    const style = typeof props.module.style === "string" ? Number(props.module.style) : props.module.style;
    return style === 2 ? 5 : 4;
});

const cssVars = computed(() => {
    const goodsPadding = Number(props.module.goodsPadding ?? 5);
    return {
        "--dec-columns": String(columns.value),
        "--dec-goods-padding": `${Number.isFinite(goodsPadding) ? goodsPadding : 5}px`
    } as Record<string, string>;
});

// 对齐后台预览：Settings 未暴露开关时，运行态默认展示名称/描述/价格
const showName = computed(() => props.module?.showName !== false && props.module?.showName !== 0);
const showBrief = computed(() => props.module?.showBrief !== false && props.module?.showBrief !== 0);
const showPrice = computed(() => props.module?.showPrice !== false && props.module?.showPrice !== 0);

// 购买按钮配置
const buyBtnConfig = computed(() => normalizeBuyBtnStyle(props.module?.buyBtnStyle));

const isReserveProduct = (p: PcDecorateProductRow): boolean => Number(p?.productType ?? 1) !== 1;
const hasMultiSpecSku = (p: PcDecorateProductRow): boolean => {
    const skuList = p?.productSku;
    if (!Array.isArray(skuList) || skuList.length === 0) return false;
    if (skuList.length > 1) return true;
    const first = skuList[0];
    return Array.isArray(first?.skuData) && first.skuData.length > 1;
};

// ProductBuy 的 id/skuId 在 NexTrade 为 Number 类型，统一转数字避免类型告警
const resolveProductId = (item: PcDecorateProductRow): number | undefined => {
    const id = item?.productId;
    return id != null && id !== "" ? Number(id) : undefined;
};

/** productSku 可能为 SKU 数组或单体（装修/接口历史差异） */
const resolveSkuId = (item: PcDecorateProductRow): number | undefined => {
    const ps = (item as Record<string, unknown>).productSku as unknown;
    if (ps == null) return undefined;
    if (Array.isArray(ps)) {
        const first = ps[0] as { skuId?: unknown } | undefined;
        return first?.skuId != null ? Number(first.skuId) : undefined;
    }
    const obj = ps as { skuId?: unknown; id?: unknown };
    if (obj.skuId != null) return Number(obj.skuId);
    if (obj.id != null) return Number(obj.id);
    return undefined;
};

const openProduct = (p: PcDecorateProductRow): void => {
    navigateTo(urlFormat({ path: "product", seoLink: p?.seoLink, sn: p?.productSn }), { open: { target: "_blank" } });
};

const priceFontStyle = computed(() => {
    return {
        color: "var(--dec-price-color, #e60012)",
        fontSize: "14px",
        fontWeight: 600
    };
});

// ===================== 数据获取方法 =====================
const _getproductList = async (): Promise<void> => {
    const param: Record<string, unknown> = {};
    const products = (props.module as PcProductModuleConfig).products;
    if (products && typeof products === "object") {
        if (products.productSelectType) {
            const type = Number(products.productSelectType);
            const keys = paramsValueMap[type];
            if (!keys) return;

            if (type === 1) {
                const rawIds = products.productIds;
                const ids = Array.isArray(rawIds) ? rawIds : typeof rawIds === "string" && rawIds ? rawIds.split(",").filter(Boolean) : [];
                if (ids.length > 0) {
                    const firstKey = keys[0];
                    if (!firstKey) return;
                    const mappedKey = paramsKeyMap[firstKey];
                    if (!mappedKey) return;
                    // NexTrade 后端 ids 接收逗号串
                    param[mappedKey] = ids.join(",");
                    param.size = ids.length;
                }
            } else {
                param.page = 1;
                keys.forEach((key: string) => {
                    const mappedKey = paramsKeyMap[key];
                    if (!mappedKey) return;
                    param[mappedKey] = products[key];
                });
            }
        }
    }
    const rawShopId = props.shopId;
    const shopIdStr = rawShopId === null || rawShopId === undefined ? "" : String(rawShopId).trim();
    if (shopIdStr && shopIdStr !== "-1" && shopIdStr !== "0") {
        param.shopId = shopIdStr;
    }
    try {
        const result = await getProductsList(param);
        productList.value = (result?.records ?? []) as unknown as PcDecorateProductRow[];
    } catch (error) {
        console.log(error);
    }
};

// 初始化加载数据
_getproductList();
</script>

<style lang="less" scoped>
.dec-pc-product {
    :deep(.base-class) {
        width: 100%;
        min-height: 1px;
        box-sizing: border-box;
        overflow: hidden; // 圆角时裁切内部内容，对齐后台预览
    }

    &__wrapper {
        padding: var(--dec-goods-padding, 5px);
    }

    &__list {
        display: grid;
        grid-template-columns: repeat(var(--dec-columns, 4), minmax(0, 1fr));
        gap: 12px; // 对齐后台预览固定间距
        width: 100%;
    }

    &__item {
        min-width: 0;
    }

    &__card {
        display: flex;
        flex-direction: column;
        height: 100%;
        border: 1px solid rgba(50, 50, 51, 0.08);
        border-radius: 6px;
        overflow: hidden;
        background: #fff;
        text-decoration: none;
        color: inherit;
    }

    &__photo {
        width: 100%;
        aspect-ratio: 1 / 1;
        background: #f5f5f5;
        overflow: hidden;
    }

    &__img {
        width: 100%;
        height: 100%;
        display: block;
    }

    :deep(.dec-pc-product__img .el-image__inner),
    :deep(.dec-pc-product__img img) {
        width: 100%;
        height: 100%;
        object-fit: cover;
        display: block;
    }

    &__info {
        display: flex;
        flex-direction: column;
        flex: 1;
        padding: 10px 10px 12px;
    }

    &__name {
        font-size: 13px;
        line-height: 18px;
        color: #333;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
    }

    &__brief {
        margin-top: 6px;
        font-size: 12px;
        line-height: 16px;
        color: #999;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
    }

    &__price {
        margin-top: 8px;
    }

    &__bottom {
        display: flex;
        align-items: center;
        justify-content: space-between;
        margin-top: auto;
    }

    &__buy {
        flex-shrink: 0;
        cursor: pointer;
    }
}
</style>
