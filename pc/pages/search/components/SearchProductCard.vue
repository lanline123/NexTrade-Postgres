<template>
    <div class="product-card-container" @click="toProduct">
        <div class="top-wrap">
            <div class="pro_pic">
                <NuxtLink :to="productLink" target="_blank" @click.stop>
                    <img :class="isSoldOut ? 'img sold-out' : 'img'" :src="productImage(product)" :alt="product.productName" loading="lazy" />
                </NuxtLink>
            </div>
        </div>
        <div class="center-wrap">
            <div class="pro_name" :class="{ 'pro_name--fixed-two-lines': fixedNameTwoLines }">
                {{ product.productName }}
            </div>

            <template v-if="isSoldOut">
                <div class="desc-wrap">
                    <div class="virtual-sales">{{ $t("已售罄") }}</div>
                </div>
            </template>
            <template v-else-if="searchConfig.showSales !== false && searchConfig.showSales !== 0 && searchConfig.showSales !== '0'">
                <div class="desc-wrap">
                    <div class="virtual-sales">{{ $t("已售") }} {{ product.virtualSales }}</div>
                </div>
            </template>

            <div v-if="activityTags.length > 0" class="activity-tags">
                <span v-for="tag in activityTags" :key="tag" class="activity-tag">{{ $t(tag) }}</span>
            </div>

            <div class="price-wrap">
                <div class="box">
                    <div class="seckill-price">
                        <FormatPrice
                            :model-value="cardPrice(product, product.price)"
                            :price-style="{ alignItems: 'baseline' }"
                            :currency-style="{ fontSize: '16px', color: priceColor, fontWeight: '400' }"
                            :font-style="{ fontSize: '28px', color: priceColor, fontWeight: '600', lineHeight: '28px' }"
                            :decimals-style="{ fontSize: '16px', color: priceColor, fontWeight: '400' }"
                        />
                    </div>
                </div>

                <!--
                    注意：外层卡片有 @click 会跳转详情页；这里必须用原生 DOM 的 click.stop
                    才能阻止"加入购物车/查看详情按钮"点击继续冒泡导致误跳转。
                -->
                <div
                    v-if="searchConfig.showCartButton !== false && searchConfig.showCartButton !== 0 && searchConfig.showCartButton !== '0'"
                    class="buy"
                    @click.stop
                >
                    <!-- 拼团商品：点击跳转拼团商品详情页 -->
                    <template v-if="groupData?.type === 7">
                        <NuxtLink :to="productLink" target="_blank" @click.stop>
                            <span class="buy-btn" :class="buyBtnClass">
                                <tig-icon :name="buyIconName" :size="28" />
                            </span>
                        </NuxtLink>
                    </template>
                    <!-- 预约/非普通商品：禁止加购，点击按钮跳转详情 -->
                    <template v-else-if="isReserveProduct">
                        <NuxtLink :to="productLink" target="_blank" @click.stop>
                            <span class="buy-btn" :class="buyBtnClass">
                                <tig-icon :name="buyIconName" :size="28" />
                            </span>
                        </NuxtLink>
                    </template>
                    <!-- 多规格：不支持直接加购，点击跳转商品详情选择规格 -->
                    <template v-else-if="product.productSku && product.productSku.length > 1">
                        <NuxtLink :to="productLink" target="_blank" @click.stop>
                            <span class="buy-btn" :class="buyBtnClass">
                                <tig-icon :name="buyIconName" :size="28" />
                            </span>
                        </NuxtLink>
                    </template>
                    <!-- 单规格：可直接加购 -->
                    <template v-else>
                        <ProductBuy
                            :id="Number(product.productId ?? 0)"
                            :shop-id="buyShopId"
                            :is-quick="isPresale"
                            :is-cycle-info="product.deliveryMode === 2"
                            :disabled="isSoldOut"
                            :number="1"
                            @callback="$emit('callback')"
                        >
                            <span class="buy-btn" :class="[buyBtnClass, { disabled: isSoldOut }]">
                                <tig-icon :name="buyIconName" :size="28" />
                            </span>
                        </ProductBuy>
                    </template>
                </div>
            </div>

            <div v-if="searchConfig.showShopTag !== false && searchConfig.showShopTag !== 0 && searchConfig.showShopTag !== '0'" class="other-wrap">
                <template v-if="product.shop">
                    <NuxtLink class="link" :to="urlFormat({ path: 'shop', id: product.shopId })" target="_blank" @click.stop>
                        {{ $t(product?.shop?.shopTitle || "") }}
                    </NuxtLink>
                </template>
                <template v-else>
                    {{ $t(commonStore.defaultShopName || "官方自营") }}
                </template>
            </div>
        </div>
    </div>
</template>

<script lang="ts" setup>
// ===================== 类型导入 =====================
import type { PropType } from "vue";
import type { SearchProductItem } from "./types.d";

// ===================== Store =====================
import { useCommonStore } from "~/store/common";
import { useProductCardBuy } from "~/hooks/useProductCardBuy";

// ===================== 工具函数 =====================
import { isO2o } from "~/utils/util";

// ===================== Props & Emits =====================
const props = defineProps({
    product: {
        type: Object as PropType<SearchProductItem>,
        required: true
    },
    groupData: {
        type: Object as PropType<Record<string, unknown>>,
        default: () => ({})
    },
    activityInfo: {
        type: Array as PropType<Array<Record<string, unknown>>>,
        default: () => []
    },
    searchConfig: {
        type: Object as PropType<Record<string, unknown>>,
        default: () => ({})
    },
    fixedNameTwoLines: {
        type: Boolean,
        default: false
    }
});

defineEmits(["callback"]);

// ===================== Store 实例 =====================
const commonStore = useCommonStore();
const { cardPrice, cardProductLink } = useProductCardBuy();

// ===================== 计算属性 =====================
/** 是否售罄 */
const isSoldOut = computed((): boolean => {
    const p = props.product;
    const seckill = p.isSeckill as unknown;
    return (seckill == 0 && p.productStock == 0) || (seckill == null && p.productStock == 0) || (seckill == 1 && p.seckillStock == 0);
});

// 约定：productType === 1 为普通商品可加购；其他类型（如预约/服务等）需要去详情页选择/预约
const isReserveProduct = computed(() => Number(props.product?.productType ?? 1) !== 1);

/** 预售标识：兼容 productSaleMode / saleMode */
const isPresale = computed((): boolean => {
    const p = props.product as Record<string, unknown>;
    const numericMode = Number(p?.productSaleMode ?? p?.saleMode);
    if (Number.isNaN(numericMode)) return false;
    return numericMode > 0;
});

/** 加购所属门店；多门店下不传会被判定为「商品不在该店售卖」 */
const buyShopId = computed<number | undefined>(() => {
    if (!isO2o()) return undefined;
    const shopId = props.product?.shopId;
    return shopId != null && shopId !== "" ? Number(shopId) : undefined;
});

/** 购买按钮样式类 */
const buyBtnClass = computed(() => {
    const style = props.searchConfig.buyStyle ?? 1;
    return {
        "buy-btn--style-1": style === 1,
        "buy-btn--style-2": style === 2,
        "buy-btn--style-3": style === 3
    };
});

/** 购买按钮图标 */
const buyIconName = computed(() => {
    const style = props.searchConfig.buyStyle ?? 1;
    if (style === 2) return "buy-cart";
    if (style === 3) return "buy-cart-circle";
    return "buy-jia";
});

/** 活动类型文字映射: 1秒杀 2优惠券 3满减 4折扣 5满赠 6限时折扣 7拼团 */
const ACTIVITY_TYPE_TEXT: Record<number, string> = {
    1: "秒杀",
    3: "满减",
    4: "折扣",
    5: "满赠",
    6: "限时折扣",
    7: "拼团"
};

const activityTags = computed((): string[] => {
    if (!Array.isArray(props.activityInfo) || props.activityInfo.length === 0) return [];
    return props.activityInfo
        .filter((item) => {
            const t = Number(item.type);
            return t !== 2 && Boolean(ACTIVITY_TYPE_TEXT[t]);
        })
        .map((item) => ACTIVITY_TYPE_TEXT[Number(item.type)]!);
});

/** 价格颜色（跟随装修主题色） */
const priceColor = computed(() => {
    return "var(--general)";
});

/** 商品详情链接：多门店场景下携带 shopId */
const productLink = computed<string>(() => {
    return cardProductLink(props.product, {
        path: "product",
        seoLink: props.product.seoLink,
        sn: props.product.productSn,
        shopId: isO2o() ? props.product.shopId : undefined
    });
});

// ===================== 事件处理方法 =====================
const toProduct = (): void => {
    navigateTo(productLink.value, { open: { target: "_blank" } });
};
</script>

<style lang="less" scoped>
.product-card-container {
    box-sizing: border-box;
    width: 100%;
    border-radius: 4px;
    padding-bottom: 12px;
    cursor: pointer;
    background: #fff;
    overflow: hidden;
    border: 1px solid transparent !important;
    transition: border-color 0.2s ease;

    &:hover {
        border: 1px solid var(--general) !important;
    }

    .top-wrap {
        .pro_pic {
            width: 100%;
            padding-top: 100%;
            height: 0;
            position: relative;
            background: #fafafa;
            a {
                position: absolute;
                inset: 0;
                width: 100%;
                height: 100%;
                overflow: hidden;
                display: inline-block;
                .img {
                    width: 100%;
                    height: 100%;
                    object-fit: contain;
                    display: block;
                    &.sold-out {
                        opacity: 0.5;
                    }
                }
            }
        }
    }

    .center-wrap {
        padding: 8px 12px 0 12px;
        margin-top: 8px;

        .pro_name {
            font-size: 16px;
            line-height: 22px;
            word-break: break-word;
            text-overflow: ellipsis;
            overflow: hidden;
            display: -webkit-box;
            line-clamp: 2;
            -webkit-line-clamp: 2;
            -webkit-box-orient: vertical;
            color: #19191b;

            &--fixed-two-lines {
                min-height: calc(22px * 2);
            }
        }

        .desc-wrap {
            .virtual-sales {
                margin-top: 10px;
                font-size: 14px;
                font-weight: 400;
                line-height: 18px;
                text-align: left;
                text-underline-position: from-font;
                text-decoration-skip-ink: none;
                color: #888b94;
                overflow: hidden;
                text-overflow: ellipsis;
                white-space: nowrap;
            }
        }

        .price-wrap {
            display: flex;
            justify-content: space-between;
            align-items: center;
            margin-top: 6px;

            .buy {
                .buy-btn {
                    display: flex;
                    justify-content: center;
                    align-items: center;
                    width: 32px;
                    height: 32px;
                    color: var(--general);
                    flex-shrink: 0;
                    cursor: pointer;
                    border: none;
                    background-color: transparent;

                    &:hover {
                        opacity: 0.8;
                    }

                    &.disabled {
                        opacity: 0.5;
                        cursor: not-allowed;
                    }

                    // 样式一：默认
                    &--style-1 {
                        background-color: transparent;
                    }

                    // 样式二：无边框
                    &--style-2 {
                        border: none;
                    }

                    // 样式三：圆角
                    &--style-3 {
                        border-radius: 6px;
                    }
                }
            }
        }

        .other-wrap {
            width: 100%;
            height: 18px;
            box-sizing: border-box;
            font-size: 14px;
            line-height: 18px;
            color: #269636;
            font-weight: 600;
            margin-top: 6px;
            overflow: hidden;
            text-overflow: ellipsis;
            white-space: nowrap;
            .link {
                font-size: 14px;
                line-height: 18px;
                color: #269636;
                font-weight: 600;
            }
        }

        .activity-tags {
            display: flex;
            flex-wrap: wrap;
            gap: 8px;
            margin-top: 12px;
            min-height: 18px;

            .activity-tag {
                font-size: 12px;
                line-height: 18px;
                color: var(--general);
                font-weight: bold;
                white-space: nowrap;
            }
        }
    }
}
</style>
