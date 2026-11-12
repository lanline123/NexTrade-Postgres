<template>
    <!-- PC 商品分组模块 -->
    <div class="dec-product-group" :class="moduleClasses" :style="cssVars">
        <div class="dec-product-group__container">
            <!-- 分组标题栏 -->
            <template v-if="showGroupTitle">
                <div class="dec-product-group__tabs-wrapper">
                    <!-- 左箭头 -->
                    <div v-if="showLeftArrow" class="dec-product-group__arrow dec-product-group__arrow--left" @click="scrollTabs('left')">
                        <span class="dec-product-group__arrow-btn">
                            <SvgIcon name="arrow-right" class="dec-product-group__arrow-icon dec-product-group__arrow-icon--flip" />
                        </span>
                    </div>
                    <div ref="tabsRef" class="dec-product-group__tabs" @scroll="updateArrows">
                        <div
                            v-for="(item, index) in tabList"
                            :key="index"
                            class="dec-product-group__tab"
                            :class="{ 'dec-product-group__tab--active': Number(index) === activeIndex }"
                            @click="handleTabClick(Number(index))"
                        >
                            <!-- 分隔线 -->
                            <div v-if="Number(index) > 0" class="dec-product-group__tab-line"></div>
                            <div class="dec-product-group__tab-content">
                                <!-- 标题图片 -->
                                <img v-if="productImage(item.titlePic)" class="dec-product-group__tab-icon" :src="productImage(item.titlePic)" alt="" />
                                <!-- 标题文字（与 showGroupTitle 用同一口径，兼容字符串 "0"/"1"） -->
                                <div v-if="showGroupTitle" class="dec-product-group__tab-title">
                                    {{ item.title }}
                                </div>
                            </div>
                        </div>
                    </div>
                    <!-- 右箭头 -->
                    <div v-if="showRightArrow" class="dec-product-group__arrow dec-product-group__arrow--right" @click="scrollTabs('right')">
                        <span class="dec-product-group__arrow-btn">
                            <SvgIcon name="arrow-right" class="dec-product-group__arrow-icon" />
                        </span>
                    </div>
                </div>
            </template>

            <!-- 商品列表 -->
            <div class="dec-product-group__products">
                <div class="dec-product-group__list">
                    <div
                        v-for="(product, index) in currentProductList"
                        :key="product.id || product.productId || index"
                        class="dec-product-group__item"
                        :style="itemWidthStyle(index)"
                    >
                        <NuxtLink
                            class="dec-product-group__card"
                            target="_blank"
                            :to="urlFormat({ path: 'product', seoLink: product.seoLink, sn: product.productSn })"
                        >
                            <!-- 商品图片 -->
                            <div class="dec-product-group__image">
                                <img class="dec-product-group__image-inner" :src="productImage(product)" :alt="product.productName" loading="lazy" />
                            </div>
                            <!-- 商品信息 -->
                            <div class="dec-product-group__info">
                                <!-- 商品名称 -->
                                <div v-if="showProductName" class="dec-product-group__name">
                                    {{ product.productName }}
                                </div>
                                <!-- 已售/状态 -->
                                <div v-if="showSales" class="dec-product-group__status-wrap">
                                    <div class="dec-product-group__sales">{{ $t("已售") }} {{ product.virtualSales ?? 0 }}</div>
                                </div>
                                <div v-if="getActivityTags(product.id || product.productId).length > 0" class="dec-product-group__tag-wrap">
                                    <span v-for="tag in getActivityTags(product.id || product.productId)" :key="tag" class="dec-product-group__tag">
                                        {{ $t(tag) }}
                                    </span>
                                </div>
                                <!-- 价格区域 -->
                                <div class="dec-product-group__price-box">
                                    <div class="dec-product-group__price-row">
                                        <!-- 价格（NexTrade 列表字段为 productPrice） -->
                                        <FormatPrice
                                            v-if="showProductPrice"
                                            v-model="product.productPrice"
                                            :price-style="{ alignItems: 'baseline' }"
                                            :currency-style="{ fontSize: '16px', color: 'var(--primary-dark-2)', fontWeight: '400' }"
                                            :font-style="{ fontSize: '28px', color: 'var(--primary-dark-2)', fontWeight: '600', lineHeight: '28px' }"
                                            :decimals-style="{ fontSize: '16px', color: 'var(--primary-dark-2)', fontWeight: '400' }"
                                        ></FormatPrice>
                                        <!-- 划线价 -->
                                        <FormatPrice
                                            v-if="showCrossedOutPrice"
                                            v-model="product.marketPrice"
                                            style="margin-left: 5px"
                                            :font-style="originPriceFontStyle"
                                            :currency-style="origincurrencyPriceFontStyle"
                                            :decimals-style="originPriceFontStyle"
                                        />
                                    </div>
                                    <!-- 购买按钮 -->
                                    <div v-if="showBuyBtn" class="dec-product-group__cart-btn" :class="buyBtnClass" @click.prevent.stop>
                                        <template v-if="isGrouponProduct(product) || isReserveProduct(product) || hasMultiSpecSku(product)">
                                            <div
                                                class="dec-product-group__cart-inner"
                                                style="width: 30px; transform: translateX(-2px)"
                                                @click.stop="openProduct(product)"
                                            >
                                                <SvgIcon :name="buyIconName" class="dec-product-group__cart-icon" />
                                            </div>
                                        </template>
                                        <template v-else>
                                            <ProductBuy
                                                :id="resolveProductId(product)"
                                                :sku-id="resolveSkuId(product)"
                                                :is-quick="false"
                                                :disabled="isSoldOut(product)"
                                                :number="1"
                                            >
                                                <div
                                                    class="dec-product-group__cart-inner"
                                                    :class="{ 'is-disabled': isSoldOut(product) }"
                                                    style="width: 30px; transform: translateX(-2px)"
                                                >
                                                    <SvgIcon :name="buyIconName" class="dec-product-group__cart-icon" />
                                                </div>
                                            </ProductBuy>
                                        </template>
                                    </div>
                                </div>
                                <!-- 商品店铺 -->
                                <div v-if="showShopTag" class="dec-product-group__shop">
                                    <template v-if="product.shop">
                                        {{ product.shop?.shopTitle }}
                                    </template>
                                    <template v-else>
                                        {{ $t(commonStore.defaultShopName) || $t("官方自营") }} 
                                    </template>
                                </div>
                            </div>
                        </NuxtLink>
                    </div>
                </div>
            </div>
        </div>
    </div>
</template>

<script lang="ts" setup>
/**
 * PC 商品分组模块 - 前台渲染组件
 * 对齐 Admin 后台预览，使用相同的 CSS 变量体系
 */

// ===================== Vue 核心 =====================
import { computed, ref, watch, onMounted, nextTick } from "vue";

// ===================== API 导入 =====================
import { getProductsList } from "@/api/common";

// ===================== Store =====================
import { useCommonStore } from "~/store/common";

// ===================== 工具函数 =====================
import { productImage, urlFormat } from "~/utils/format";
import { usePromotionActivity } from "~/composables/usePromotionActivity";
import type { PcDecorateProductRow } from "~/types/decoration/pcDecorateProduct";

interface ProductGroupTabRow {
    titlePic?: { picUrl?: string };
    title?: string;
}

interface PcProductGroupModuleData {
    productGroups?: Array<Record<string, unknown>>;
    groupTitle?: { showTitle?: number };
    showProductName?: number;
    showSales?: number;
    showProductPrice?: number;
    crossedOutPrice?: unknown;
    productCardStyle?: number;
    listStyle?: number;
    showShopTag?: number;
    cardPadding?: number;
    cardRadius?: number;
    productNameWeight?: number;
    productNameSize?: number;
    productNameRow?: number;
    buyBtnStyle?: { showBtn?: number; btnType?: number };
    productColor?: Record<string, unknown>;
    moduleStyle?: Record<string, unknown>;
    contentStyle?: Record<string, unknown>;
}

// ===================== Props =====================
const props = defineProps<{
    module: Record<string, unknown>;
}>();

// ===================== Store =====================
const commonStore = useCommonStore();

// ===================== 数据解构 =====================
const data = computed(() => (props.module || {}) as PcProductGroupModuleData);
const DEFAULT_PRICE_WEIGHT = 400;
const DEFAULT_PRICE_INT_SIZE = 28;
const DEFAULT_PRICE_MARGIN_TOP = 10;

const itemGap = computed(() => {
    const gap = Number(data.value?.productColor?.infoLeftRightPadding ?? 3);
    if (!Number.isFinite(gap)) return 3;
    return Math.max(0, gap);
});

// 显示开关统一做数字归一化：装修数据从后端返回时可能为字符串 "0"/"1"，
// 严格比较（=== 1 / !== 0）会失效，故统一 Number() 转换后判断（缺省按显示处理）
const showProductName = computed(() => Number(data.value.showProductName ?? 1) !== 0);
const showProductPrice = computed(() => Number(data.value.showProductPrice ?? 1) !== 0);
const showCrossedOutPrice = computed(() => Number(data.value.crossedOutPrice ?? 1) !== 0);
const showSales = computed(() => Number(data.value.showSales ?? 1) !== 0);
const showShopTag = computed(() => Number(data.value.showShopTag ?? 1) !== 0);
const showBuyBtn = computed(() => Number(data.value.buyBtnStyle?.showBtn ?? 1) !== 0);

// 当前激活的分组索引
const activeIndex = ref(0);

// ========== 箭头滚动 ==========
const tabsRef = ref<HTMLElement | null>(null);
const showLeftArrow = ref(false);
const showRightArrow = ref(false);

function updateArrows() {
    const el = tabsRef.value;
    if (!el) {
        showLeftArrow.value = false;
        showRightArrow.value = false;
        return;
    }
    showLeftArrow.value = el.scrollLeft > 0;
    showRightArrow.value = el.scrollLeft + el.clientWidth < el.scrollWidth - 1;
}

function scrollTabs(direction: "left" | "right") {
    const el = tabsRef.value;
    if (!el) return;
    const step = el.clientWidth * 0.6;
    el.scrollBy({ left: direction === "left" ? -step : step, behavior: "smooth" });
}

// ===================== 计算属性 =====================

/** Tab 列表 */
const tabList = computed((): ProductGroupTabRow[] => (Array.isArray(data.value.productGroups) ? (data.value.productGroups as ProductGroupTabRow[]) : []));

watch(tabList, () => {
    nextTick(updateArrows);
});

onMounted(() => {
    nextTick(updateArrows);
});

const showGroupTitle = computed(() => Number(data.value.groupTitle?.showTitle ?? 1) === 1);

/** 当前分组的商品列表 */
const currentProductList = ref<PcDecorateProductRow[]>([]);
const { getActivityInfo, fetchPromotionList, clearPromotionList } = usePromotionActivity();

/** 模块类名 */
const moduleClasses = computed(() => {
    const d = data.value;
    return [`dec-product-group--card-style-${d.productCardStyle ?? 1}`, `dec-product-group--list-style-${d.listStyle ?? 4}`];
});

/** CSS 变量 */
const cssVars = computed(() => {
    const d = data.value;
    const pc = (d.productColor || {}) as Record<string, string | number | undefined>;
    const gt = (d.groupTitle || {}) as Record<string, string | number | undefined>;
    const ms = (d.moduleStyle || {}) as Record<string, string | number | boolean | undefined>;
    const cs = (d.contentStyle || {}) as Record<string, string | number | boolean | undefined>;

    return {
        // 模块样式
        "--dec-box-padding": `${ms.boxPadding ?? 10}px`,
        "--dec-box-padding-top": `${ms.boxPaddingTop ?? 5}px`,
        "--dec-box-padding-bottom": `${ms.boxPaddingBottom ?? 5}px`,
        "--dec-module-bg": String(ms.backgroundColor ?? "transparent"),

        // 内容样式
        "--dec-content-padding-top": `${cs.paddingTop ?? 0}px`,
        "--dec-content-padding-bottom": `${cs.paddingBottom ?? 0}px`,
        "--dec-content-padding-lr": `${cs.paddingLeftRight ?? 0}px`,
        "--dec-content-radius": `${cs.boxRadius ?? 0}px`,
        "--dec-content-bg": String(cs.backgroundColor ?? "transparent"),

        // 商品卡片样式
        "--dec-card-padding": `${d.cardPadding ?? 0}px`,
        "--dec-card-radius": `${d.cardRadius ?? 4}px`,
        "--dec-card-bg": String(pc.backgroundColor ?? "#ffffff"),

        // 商品信息样式
        "--dec-item-gap": `${itemGap.value}px`,
        "--dec-info-padding-lr": `${pc.infoLeftRightPadding ?? 3}px`,
        "--dec-title-color": String(pc.titleColor ?? "#19191b"),
        "--dec-title-weight": String(d.productNameWeight ?? 400),
        "--dec-title-size": `${d.productNameSize ?? 16}px`,
        "--dec-title-rows": String(d.productNameRow ?? 1),
        "--dec-price-color": String(pc.priceColor ?? "var(--vort-primary, #ff4d4f)"),
        "--dec-price-weight": String(DEFAULT_PRICE_WEIGHT),
        "--dec-price-size": `${DEFAULT_PRICE_INT_SIZE}px`,
        "--dec-price-margin-top": `${DEFAULT_PRICE_MARGIN_TOP}px`,
        "--dec-origin-price-color": String(pc.crossedOutPriceColor ?? "#999999"),

        // 分组标题样式
        "--dec-tab-margin-lr": `${gt.marginLeftRight ?? 0}px`,
        "--dec-tab-title-size": `${gt.bigTitleFontSize ?? 14}px`,
        "--dec-tab-title-color": String(gt.titleColor ?? "#444444"),
        "--dec-tab-title-active-color": String(gt.activeTitleColor ?? "#FF0F23"),
        "--dec-tab-active-bg": String(gt.activeTitleBgColor ?? "#ffebf1"),
        "--dec-tab-title-min-width": `${gt.titleMinWidth ?? 80}px`,
        "--dec-tab-title-padding-tb": `${gt.titlePaddingTopBottom ?? 8}px`,
        "--dec-tab-title-padding-lr": `${gt.titlePaddingLeftRight ?? 12}px`,
        "--dec-tab-title-radius": `${gt.titleRadius ?? 4}px`,
        "--dec-tab-line-color": String(gt.splitLineColor ?? "#DADADA")
    } satisfies Record<string, string>;
});

const originPriceFontStyle = computed(() => {
    return {
        color: "var(--dec-origin-price-color, #999999)",
        fontSize: "16px",
        fontWeight: 400,
        textDecoration: "line-through"
    };
});

const origincurrencyPriceFontStyle = computed(() => {
    return {
        color: "var(--dec-origin-price-color, #999999)",
        fontSize: "12px",
        fontWeight: 400
    };
});

/** 购买按钮样式类（btnType 可能为字符串，统一 Number 转换） */
const buyBtnClass = computed(() => {
    const style = Number(data.value.buyBtnStyle?.btnType ?? 1);
    return {
        "dec-product-group__cart-btn--style-1": style === 1,
        "dec-product-group__cart-btn--style-2": style === 2,
        "dec-product-group__cart-btn--style-3": style === 3
    };
});

/** 购买按钮图标名 */
const buyIconName = computed(() => {
    const style = Number(data.value.buyBtnStyle?.btnType ?? 1);
    if (style === 2) return "buy-cart";
    if (style === 3) return "buy-cart-circle";
    return "buy-jia";
});

/** 活动类型文字映射（仅白名单展示；type=2 券走独立样式，type=8 抽奖不展示） */
const ACTIVITY_TYPE_TEXT: Record<number, string> = {
    1: "秒杀",
    3: "满减",
    4: "满折",
    5: "满赠",
    6: "限时折扣",
    7: "拼团"
};

function isGrouponProduct(product: PcDecorateProductRow): boolean {
    const pid = product.productId ?? product.id;
    if (pid == null || pid === "") return false;
    const activityInfo = getActivityInfo(pid) as Array<{ type?: number }>;
    return activityInfo.some((item) => item?.type === 7);
}

function getActivityTags(productId: string | number | undefined): string[] {
    if (productId == null || productId === "") return [];
    const activityInfo = getActivityInfo(productId) as Array<{ type?: number; typeText?: string }>;
    const tags: string[] = [];

    activityInfo.forEach((item) => {
        if (typeof item?.type !== "number" || item.type === 2) return;
        const fallback = ACTIVITY_TYPE_TEXT[item.type];
        if (typeof fallback !== "string") return;
        const typeText = typeof item.typeText === "string" ? item.typeText.trim() : "";
        tags.push(typeText || fallback);
    });

    return tags;
}

/** 计算商品项宽度 */
const itemWidthStyle = (index: number) => {
    const listStyle = Number(data.value.listStyle ?? 4);
    const gap = `${itemGap.value}px`;
    if (listStyle === 5) {
        return {
            width: index % 3 === 0 ? "100%" : `calc((100% - ${gap}) / 2)`
        };
    }
    if (listStyle === 6) {
        return { width: `calc((100% - (${gap} * 4)) / 5)` };
    }
    if (listStyle === 7) {
        return { width: `calc((100% - (${gap} * 5)) / 6)` };
    }
    if (listStyle >= 1 && listStyle <= 4) {
        return { width: `calc((100% - (${gap} * ${listStyle - 1})) / ${listStyle})` };
    }
    return {};
};

/** 根据展示样式获取每行列数 */
function getColumnsCount(): number {
    const listStyle = Number(data.value.listStyle ?? 4);
    const map: Record<number, number> = { 4: 4, 6: 5, 7: 6 };
    return map[listStyle] || 4;
}

/** 判断商品是否售罄 */
function isSoldOut(product: PcDecorateProductRow): boolean {
    const p = product;
    return (p?.isSeckill == 0 && p?.productStock == 0) || (p?.isSeckill == undefined && p?.productStock == 0) || (p?.isSeckill == 1 && p?.seckillStock == 0);
}

function isReserveProduct(product: PcDecorateProductRow): boolean {
    return Number(product?.productType ?? 1) !== 1;
}

function hasMultiSpecSku(product: PcDecorateProductRow): boolean {
    const skuList = product?.productSku;
    if (!Array.isArray(skuList) || skuList.length === 0) return false;
    if (skuList.length > 1) return true;
    const first = skuList[0];
    return Array.isArray(first?.skuData) && first.skuData.length > 1;
}

// ProductBuy 的 id/skuId 在 NexTrade 为 Number 类型，统一转数字避免类型告警
function resolveProductId(product: PcDecorateProductRow): number | undefined {
    const id = product?.productId ?? product?.id;
    return id != null && id !== "" ? Number(id) : undefined;
}

function resolveSkuId(product: PcDecorateProductRow): number | undefined {
    const sku = product?.productSku?.[0];
    const skuId = sku?.skuId ?? sku?.id;
    return skuId != null ? Number(skuId) : undefined;
}

function openProduct(product: PcDecorateProductRow): void {
    navigateTo(urlFormat({ path: "product", seoLink: product?.seoLink, sn: product?.productSn }), { open: { target: "_blank" } });
}

// ===================== Tab 切换 =====================
const handleTabClick = (index: number) => {
    activeIndex.value = index;

    // 滚动使点击的 tab 居中显示
    nextTick(() => {
        const container = tabsRef.value;
        if (!container) return;
        const tabEl = container.children[index] as HTMLElement;
        if (!tabEl) return;

        const containerRect = container.getBoundingClientRect();
        const tabRect = tabEl.getBoundingClientRect();
        // 计算 tab 中心与容器中心的偏移量
        const tabCenter = (tabRect.left + tabRect.right) / 2 - containerRect.left;
        const containerCenter = container.clientWidth / 2;
        container.scrollBy({ left: tabCenter - containerCenter, behavior: "smooth" });
    });
};

// ===================== 数据获取 =====================
const fetchProductList = async () => {
    const d = data.value;
    const index = activeIndex.value;
    const productGroups = Array.isArray(d.productGroups) ? d.productGroups : [];
    clearPromotionList();

    if (!productGroups.length || !productGroups[index]) {
        currentProductList.value = [];
        return;
    }

    const group = productGroups[index] as Record<string, unknown>;
    const sourceType = group.sourceType || "products";

    if (sourceType === "productGroup") {
        if (!group.productGroupId) {
            currentProductList.value = [];
            return;
        }
        try {
            const cols = getColumnsCount();
            const result = await getProductsList({
                productGroupId: group.productGroupId,
                page: 1,
                size: cols * 3
            });
            currentProductList.value = (result?.records ?? []) as unknown as PcDecorateProductRow[];
            await fetchPromotionList(currentProductList.value);
        } catch {
            currentProductList.value = [];
            clearPromotionList();
        }
        return;
    }

    const fromList = Array.isArray(group.productList) ? (group.productList as Record<string, unknown>[]) : [];
    const productIds = fromList
        .map((p) => (p.id ?? p.productId) as string | number | undefined)
        .filter((id: string | number | undefined): id is string | number => Boolean(id) && String(id) !== "0");
    if (productIds && productIds.length > 0) {
        try {
            // NexTrade 后端 ids 接收逗号串
            const result = await getProductsList({ ids: productIds.join(","), size: productIds.length });
            currentProductList.value = (result?.records ?? []) as unknown as PcDecorateProductRow[];
            await fetchPromotionList(currentProductList.value);
        } catch {
            // 接口异常时，降级使用装修数据中已有的商品信息
            currentProductList.value = fromList as unknown as PcDecorateProductRow[];
            await fetchPromotionList(currentProductList.value);
        }
    } else {
        currentProductList.value = [];
        clearPromotionList();
    }
};

// 监听分组变化 + tab 切换
watch(
    [() => data.value.productGroups, activeIndex],
    () => {
        fetchProductList();
    },
    { immediate: true, deep: true }
);
</script>

<style lang="less" scoped>
@import "./PcProductGroup.less";
</style>
