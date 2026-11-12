<template>
    <!-- <NuxtLink :to="urlFormat({ path: 'product', id: product.productId, sn: product.productSn })" target="_blank"> -->
    <div class="product-card-container" @click="toProduct">
        <div class="top-wrap">
            <div class="pro_pic">
                <NuxtLink @click.stop :to="productLink" target="_blank">
                    <el-image
                        fit="contain"
                        class="img"
                        :class="{
                            'sold-out':
                                (product?.isSeckill == 0 && product?.productStock == 0) ||
                                (product?.isSeckill == undefined && product?.productStock == 0) ||
                                (product?.isSeckill == 1 && product?.seckillStock == 0)
                        }"
                        :src="productImage(product)"
                        lazy
                        :alt="product.productName || ''"
                    />
                </NuxtLink>
                <img v-if="productStyle?.borderImage" class="product-style-border" :src="productStyle.borderImage" />
            </div>
            <div class="more" v-if="isMore" @click.stop>
                <el-popover popper-class="del-pop" placement="top" :show-arrow="false" trigger="click">
                    <template #reference>
                        <SvgIcon name="more" width="20px" height="20px"></SvgIcon>
                    </template>
                    <template #default>
                        <div class="del" @click.stop.prevent="handleDelete(product)">{{ $t("删除") }}</div>
                    </template>
                </el-popover>
            </div>
        </div>
        <div class="center-wrap">
            <!-- <NuxtLink :to="urlFormat({ path: 'product', id: product.productId, sn: product.productSn })" target="_blank"> -->
            <div class="pro_name">
                <span v-if="isPresale" class="presale-text">{{ $t("预售") }}</span>
                {{ product.productName }}
            </div>
            <!-- </NuxtLink> -->
            <!-- product?.virtualSales && product?.virtualSales > 0 && product?.productStock != 0 -->
            <template v-if="isSoldOut">
                <div class="desc-wrap">
                    <div class="virtual-sales">{{ $t("已售罄") }}</div>
                </div>
            </template>
            <template v-else>
                <div class="desc-wrap">
                    <div class="virtual-sales">{{ $t("已售") }} {{ product?.virtualSales }}</div>
                </div>
            </template>

            <!-- <div class="count-down" v-if="product?.isSeckill == 1">
                    <div class="label">{{ $t("秒杀") }}</div>
                    <CountDown countdown-type="specifyTime" :specify-time="getTime(product?.seckillEndTime)">
                        <template #default="{ days, hours, minutes, seconds }">
                            <div class="countdown-box">
                                <span class="countdown-text">{{ $t("距结束") }}</span>
                                <template v-if="days && Number(days) > 0">
                                    <span class="countdown-item">{{ days }}</span>
                                    <span class="countdown-separator-text">{{ $t("天") }}</span>
                                </template>
                                <span class="countdown-item">{{ hours }}</span>
                                <span class="countdown-separator">:</span>
                                <span class="countdown-item">{{ minutes }}</span>
                                <span class="countdown-separator">:</span>
                                <span class="countdown-item">{{ seconds }}</span>
                            </div>
                        </template>
                    </CountDown>
                </div> -->
            <template v-if="groupData.type === 7">
                <span class="tag">{{ $t("拼团") }}</span>
            </template>
            <template v-else>
                <div style="height: 24px"></div>
            </template>
            <div class="price-wrap">
                <!-- v-if="product?.isSeckill == 1" -->
                <!-- <template > -->
                <div class="box">
                    <div class="seckill-price">
                        <FormatPrice
                            :model-value="cardPrice(product, product.price)"
                            :price-style="{ alignItems: 'baseline' }"
                            :currencyStyle="{ fontSize: '16px', color: 'var(--primary-dark-2)', fontWeight: '400' }"
                            :font-style="{ fontSize: '28px', color: 'var(--primary-dark-2)', fontWeight: '600', lineHeight: '28px' }"
                            :decimals-style="{ fontSize: '16px', color: 'var(--primary-dark-2)', fontWeight: '400' }"
                        ></FormatPrice>
                    </div>
                    <!-- <div class="original-price">
                                <FormatPrice
                                    :model-value="product?.orgProductPrice"
                                    :price-style="{ alignItems: 'baseline', textDecoration: 'line-through' }"
                                    :currencyStyle="{ fontSize: '16px', color: '#888b94', fontWeight: '600' }"
                                    :font-style="{ fontSize: '16px', color: '#888b94', fontWeight: '600', lineHeight: '20px' }"
                                    :decimals-style="{ fontSize: '16px', color: '#888b94', fontWeight: '600' }"
                                ></FormatPrice>
                                <div class="tips">
                                    {{ $t("优惠") }}
                                    {{ getDiscount(Number(product.price), Number(product.orgProductPrice)) }}%
                                </div>
                            </div> -->
                </div>
                <!-- </template> -->
                <!-- <template v-else>
                        <FormatPrice
                            v-model="product.productPrice"
                            :price-style="{ alignItems: 'baseline' }"
                            :currencyStyle="{ fontSize: '16px', color: '#1a1a1a', fontWeight: '400' }"
                            :font-style="{ fontSize: '28px', color: '#1a1a1a', fontWeight: '600', lineHeight: '28px' }"
                            :decimals-style="{ fontSize: '16px', color: '#1a1a1a', fontWeight: '400' }"
                        ></FormatPrice>
                    </template> -->
                <div class="buy">
                    <template
                        v-if="
                            (isWholesaleCard(product) && !canCardBuyWholesale(product)) ||
                            (!isWholesaleCard(product) && ((product.productSku && product.productSku.length > 1) || [7, 8].includes(product.productType as number)))
                        "
                    >
                        <NuxtLink @click.stop :to="productLink" target="_blank">
                            <div class="view_detail" :class="{ disabled: product?.isSeckill == 1 && product?.seckillStock == 0 }">
                                <SvgIcon name="add" width="28px" height="28px"></SvgIcon>
                            </div>
                        </NuxtLink>
                    </template>
                    <template v-else>
                        <ProductBuy
                            @click.stop
                            :isQuick="isPresale"
                            :id="product.productId"
                            :shop-id="product.shopId"
                            :disabled="(product?.isSeckill == 0 && product?.productStock == 0) || (product?.isSeckill == 1 && product?.seckillStock == 0)"
                            :number="cardBuyNumber(product)"
                            @callback="$emit('callback')"
                        >
                            <div class="view_detail" :class="{ disabled: product?.isSeckill == 1 && product?.seckillStock == 0 }">
                                <SvgIcon name="add" width="28px" height="28px"></SvgIcon>
                            </div>
                        </ProductBuy>
                    </template>
                </div>
            </div>
            <div class="other-wrap">
                <template v-if="product.shop">
                    <NuxtLink @click.stop class="link" :to="urlFormat({ path: 'shop', id: product.shopId })" target="_blank">{{
                        $t(product?.shop?.shopTitle || "")
                    }}</NuxtLink>
                </template>
                <template v-else>
                    {{ $t(commonStore.defaultShopName || "官方自营") }}
                </template>
            </div>
        </div>
    </div>
    <!-- </NuxtLink> -->
</template>
<script lang="ts" setup>
import { ref, computed, type PropType } from "vue";
import { useCommonStore } from "@/store/common";
import type { ProductsListResult } from "@/types/common/common";
import type { ProductItem } from "@/types/product/product.d";
import type { ProductActiveStyleVO } from "~/types/product/productStyle";
import CountDown from "~/components/common/CountDown.vue";
import { useProductCardBuy } from "@/hooks/useProductCardBuy";

// 卡片默认加购数量 / 卡片价格取值（批发→firstWholesalePrice，零售→fallback）
const { isWholesaleCard, canCardBuyWholesale, cardBuyNumber, cardPrice } = useProductCardBuy();

const props = defineProps({
    product: {
        type: Object as PropType<ProductsListResult | ProductItem>,
        default: () => ({})
    },
    isMore: {
        type: Boolean,
        default: false
    },
    isSearch: {
        type: Boolean,
        default: false
    },
    groupData: {
        type: Object,
        default: () => ({})
    },
    productStyle: {
        type: Object as () => ProductActiveStyleVO | null,
        default: null
    }
});

const emit = defineEmits(["delete", "callback"]);
const commonStore = useCommonStore();
const router = useRouter();
const { t } = useI18n();

/** 预售标识：兼容 productSaleMode / saleMode / presaleInfo.saleMode */
const isPresale = computed(() => {
    const p: any = props.product;
    const mode = p?.productSaleMode ?? p?.saleMode ?? p?.presaleInfo?.saleMode;
    const numericMode = Number(mode);
    if (Number.isNaN(numericMode)) return false;
    return numericMode > 0;
});

const isSoldOut = computed(() => {
    const p: any = props.product;
    return (p?.isSeckill == 0 && p?.productStock == 0) || (p?.isSeckill == undefined && p?.productStock == 0) || (p?.isSeckill == 1 && p?.seckillStock == 0);
});

const productLink = computed(() =>
    urlFormat({ path: "product", seoLink: props.product.seoLink, sn: props.product.productSn, shopId: props.product.shopId })
);

const handleDelete = (product: ProductsListResult | ProductItem) => {
    emit("delete", product);
};

const getTime = (date: string) => {
    return new Date(date).getTime();
};
const getDiscount = (currentPrice: number, originalPrice: number) => {
    if (originalPrice === 0 || currentPrice >= originalPrice) {
        return 0;
    }

    const discountPercentage = ((originalPrice - currentPrice) / originalPrice) * 100;

    return Math.floor(discountPercentage * 10) / 10;
};

const toProduct = () => {
    navigateTo(productLink.value, { open: { target: "_blank" } });
};
</script>
<style lang="less" scoped>
.product-card-container {
    box-sizing: border-box;
    width: 240px;
    // height: 408px;
    border-radius: 4px;
    padding-bottom: 12px;
    cursor: pointer;
    background: #fff;
    border-radius: 4px;
    overflow: hidden;
    border: 1px solid transparent;
    &:hover {
        border: 1px solid var(--main-bg);
    }
    .top-wrap {
        position: relative;
        .pro_pic {
            position: relative;
            width: 240px;
            height: 240px;
            background: #fafafa;
            a {
                width: 100%;
                height: 100%;
                overflow: hidden;
                display: inline-block;
                .img {
                    width: 100%;
                    height: 100%;
                    &.sold-out {
                        opacity: 0.5;
                    }
                }
            }
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
        .more {
            display: flex;
            justify-content: center;
            align-items: center;
            position: absolute;
            top: 2px;
            right: 5px;
            width: 36px;
            height: 36px;
            border-radius: 50%;
            color: #1a1a1a;
            &:hover {
                background: rgba(0, 0, 0, 0.04);
                color: var(--primary-dark-2);
            }
        }
    }
    .center-wrap {
        padding: 8px 12px 0 12px;
        margin-top: 8px;
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
        .pro_name {
            // height: 44px;
            font-size: 16px;
            font-weight: 400;
            line-height: 22px;
            vertical-align: top;
            word-break: break-word;
            text-align: left;
            font-weight: 400;
            text-overflow: -o-ellipsis-lastline;
            overflow: hidden;
            text-overflow: ellipsis;
            display: -webkit-box;
            -webkit-line-clamp: 2;
            line-clamp: 2;
            -webkit-box-orient: vertical;
            color: #19191b;

            .presale-text {
                display: inline-block;
                font-weight: bold;
                font-size: 16px;
                color: #ffffff;
                border-radius: 4px;
                background-color: #e37200;
                padding: 0px 6px;
                margin-right: 6px;
            }
            .presale-sep {
                color: #999;
                margin-right: 6px;
            }
        }

        .count-down {
            display: flex;
            align-items: center;
            width: 100%;
            height: 18px;
            box-sizing: border-box;
            font-size: 14px;
            line-height: 18px;
            text-underline-position: from-font;
            text-decoration-skip-ink: none;
            margin-top: 6px;
            text-align: left;
            .label {
                margin-right: 3px;
                font-size: 14px;
                font-weight: 600;
                color: #cc6b0a;
                text-transform: capitalize;
            }
            .countdown-box {
                font-size: 14px;
                font-weight: 500;
                color: #888b94;
                .countdown-text {
                    margin-right: 3px;
                    text-transform: capitalize;
                }
            }
        }

        .price-wrap {
            display: flex;
            justify-content: space-between;
            align-items: center;
            margin-top: 6px;
            .box {
                min-width: 0;
                overflow: hidden;
            }
            .seckill-price {
                display: flex;
                align-items: baseline;
                min-width: 0;
                overflow: hidden;
                white-space: nowrap;
            }
            .buy {
                .view_detail {
                    display: flex;
                    justify-content: center;
                    align-items: center;
                    width: 32px;
                    height: 32px;
                    border-radius: 50%;
                    background: var(--primary-light-9);
                    color: var(--main-bg);
                    flex-shrink: 0;
                    &:hover {
                        color: var(--primary-dark-2);
                    }
                    &.disabled {
                        opacity: 0.5;
                        cursor: not-allowed;
                    }
                }
            }
            .box {
                .original-price {
                    display: flex;
                    align-items: center;
                    .tips {
                        margin-left: 3px;
                        font-size: 16px;
                        color: #888b94;
                        font-weight: 500;
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
            text-underline-position: from-font;
            text-decoration-skip-ink: none;
            margin-top: 6px;
            text-align: left;
            overflow: hidden;
            text-overflow: ellipsis;
            white-space: nowrap;
            text-transform: capitalize;
            .link {
                font-size: 14px;
                line-height: 18px;
                color: #269636;
                font-weight: 600;
                &:hover {
                    color: #269636 !important;
                }
            }
        }
    }
}

.tag {
    background: #ee7700;
    color: #fff;
    padding: 2px 6px;
    border-radius: 2px;
    display: inline-block;
    margin-top: 5px;
}
</style>
<style lang="less">
.del-pop {
    display: flex;
    justify-content: center;
    width: 78px !important;
    min-width: 78px !important;
    height: 47px;
    .del {
        cursor: pointer;
        font-size: 14px;
        color: #1a1a1a;
        &:hover {
            color: #ff3f4f;
        }
    }
}
</style>
