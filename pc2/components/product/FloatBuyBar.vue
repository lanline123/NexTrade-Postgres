<template>
    <ClientOnly>
        <Teleport to="body">
            <Transition name="float-bar-slide">
                <div class="float-buy-bar" v-show="visible" :style="[themeStore.themeColor, barStyle]">
                    <div class="float-buy-bar__panel" ref="panelRef" :class="{ 'float-buy-bar__panel--docked': isDocked }" :style="panelStyle">
                        <!-- 批发态：加入购物车 + 立即购买 + 立即询价（数量/规格由右侧批发选购区维护） -->

                        <!-- B2B 批量采购：批量加入购物车 + 立即购买（isBatch 仅在 b2b 批量采购下为 true） -->
                        <template v-if="isBatch">
                            <div class="pill split">
                                <ProductBuy
                                    class="pill-buy"
                                    :shop-id="shopId"
                                    :extraAttrIds="extraAttrIds"
                                    :id="product.productId"
                                    :skuItem="skuItem"
                                    :disabled="!skuItem || skuItem.length === 0"
                                >
                                    <div class="pill__btn pill__btn--cart" :class="{ 'pill__btn--disabled': !skuItem || skuItem.length === 0 }">
                                        {{ $t("批量加入购物车") }}
                                    </div>
                                </ProductBuy>
                                <ProductBuy
                                    class="pill-buy"
                                    :shop-id="shopId"
                                    :extraAttrIds="extraAttrIds"
                                    :id="product.productId"
                                    :skuItem="skuItem"
                                    :disabled="!skuItem || skuItem.length === 0"
                                    :isQuick="true"
                                >
                                    <div class="pill__btn pill__btn--buy" :class="{ 'pill__btn--disabled': !skuItem || skuItem.length === 0 }">
                                        {{ $t("立即购买") }}
                                    </div>
                                </ProductBuy>
                            </div>
                            <!-- 立即询价：独立白底黑边黑字按钮 -->
                            <div class="float-buy-bar__inquiry" v-if="showEnquiryBtn" @click="$emit('enquiry')">{{ $t("立即询价") }}</div>
                        </template>
                        <template v-else>

                        <!-- 拼团活动：单独购买 + 我要开团/参与拼团 -->
                        <template v-if="isGroupon">
                            <div class="pill split">
                                <ProductBuy
                                    class="pill-buy"
                                    :shop-id="shopId"
                                    :extraAttrIds="extraAttrIds"
                                    :id="product.productId"
                                    :skuId="skuId"
                                    :number="number"
                                    :isQuick="true"
                                >
                                    <div class="pill__btn pill__btn--cart">{{ $t("单独购买") }}</div>
                                </ProductBuy>
                                <!-- 二期会员标签：不在人群范围只置灰开团/参团，单独购买仍走普通价可下单 -->
                                <div
                                    v-if="memberRestricted"
                                    class="pill__btn pill__btn--buy pill__btn--disabled"
                                    :title="restrictTip || $t('该商品仅限指定会员购买')"
                                >
                                    {{ grouponType === "invited" ? $t("参与拼团") : $t("我要开团") }}
                                </div>
                                <ProductBuy
                                    v-else
                                    class="pill-buy"
                                    :shop-id="shopId"
                                    :extraAttrIds="extraAttrIds"
                                    :id="product.productId"
                                    :skuId="skuId"
                                    :number="number"
                                    :isQuick="true"
                                    :disabled="!isJoin"
                                    :grouponDetail="grouponBuyDetail"
                                >
                                    <div class="pill__btn pill__btn--buy" :class="{ 'pill__btn--disabled': !isJoin }">
                                        {{ grouponType === "invited" ? $t("参与拼团") : $t("我要开团") }}
                                    </div>
                                </ProductBuy>
                            </div>
                        </template>

                        <!-- 已购买（付费内容） -->
                        <template v-else-if="product.productType === 4 && product.isBuy === 1">
                            <div class="pill pill--disabled">{{ $t("商品已购买") }}</div>
                        </template>

                        <!-- 积分兑换 -->
                        <template v-else-if="productType === 'exchange'">
                            <ProductBuy
                                class="pill-buy"
                                :shop-id="shopId"
                                :extraAttrIds="extraAttrIds"
                                :id="exchangeDetail.id"
                                :skuId="skuId"
                                :number="number"
                                :productType="productType"
                                :disabled="isOutOfStock"
                                :isQuick="true"
                            >
                                <div class="pill pill--buy" :class="{ 'pill--disabled': isOutOfStock }">{{ $t("立即兑换") }}</div>
                            </ProductBuy>
                        </template>

                        <!-- 预售 -->
                        <template v-else-if="isPresale">
                            <ProductBuy
                                class="pill-buy"
                                :shop-id="shopId"
                                :extraAttrIds="extraAttrIds"
                                :id="product.productId"
                                :skuId="skuId"
                                :number="number"
                                :disabled="isOutOfStock"
                                :isQuick="true"
                            >
                                <div class="pill pill--buy" :class="{ 'pill--disabled': isOutOfStock }">
                                    {{ presaleDetail.saleMode === 2 ? $t("立即预定") : $t("立即预购") }}
                                </div>
                            </ProductBuy>
                        </template>

                        <!-- 二期会员标签：不在人群范围；以及缺货（非预约商品）：按钮置灰，文案与可购买态保持一致 -->
                        <template v-else-if="memberRestricted || (isOutOfStock && !isReservation)">
                            <div class="pill split" :title="memberRestricted ? restrictTip || $t('该商品仅限指定会员购买') : ''">
                                <div
                                    class="pill__btn pill__btn--cart pill__btn--disabled"
                                    v-if="product.productType === 1 && !isCycleInfo"
                                >
                                    {{ $t("加入购物车") }}
                                </div>
                                <div class="pill__btn pill__btn--buy pill__btn--disabled">
                                    {{ $t(isReservation ? "立即预约" : "立即购买") }}
                                </div>
                            </div>
                            <!-- 立即询价：独立白底黑边黑字按钮；会员限制时同步置灰，缺货时仍可询价 -->
                            <div
                                class="float-buy-bar__inquiry"
                                :class="{ 'float-buy-bar__inquiry--disabled': memberRestricted }"
                                :title="memberRestricted ? restrictTip || $t('该商品仅限指定会员购买') : ''"
                                v-if="showEnquiryBtn"
                                @click="!memberRestricted && $emit('enquiry')"
                            >
                                {{ $t("立即询价") }}
                            </div>
                        </template>

                        <!-- 普通商品：加入购物车 + 立即购买/预约 -->
                        <template v-else>
                            <div class="pill split">
                                <ProductBuy
                                    v-if="product.productType === 1 && !isCycleInfo"
                                    class="pill-buy"
                                    :shop-id="shopId"
                                    :extraAttrIds="extraAttrIds"
                                    :id="product.productId"
                                    :skuId="skuId"
                                    :number="number"
                                    :disabled="isOutOfStock"
                                >
                                    <div class="pill__btn pill__btn--cart">{{ $t("加入购物车") }}</div>
                                </ProductBuy>
                                <ProductBuy
                                    class="pill-buy"
                                    :shop-id="shopId"
                                    :extraAttrIds="extraAttrIds"
                                    :id="product.productId"
                                    :skuId="skuId"
                                    :number="number"
                                    :disabled="isOutOfStock"
                                    :isQuick="true"
                                    :isCycleInfo="isCycleInfo"
                                >
                                    <div class="pill__btn pill__btn--buy">
                                        {{ $t(isReservation ? "立即预约" : "立即购买") }}
                                    </div>
                                </ProductBuy>
                            </div>
                            <!-- 立即询价：独立白底黑边黑字按钮 -->
                            <div class="float-buy-bar__inquiry" v-if="showEnquiryBtn" @click="$emit('enquiry')">{{ $t("立即询价") }}</div>
                        </template>
                        </template>

                        <RawImageDownloadButton />
                        <!-- 收藏 -->
                        <div class="float-buy-bar__collect" v-if="showCollect" @click="toggleCollect">
                            <svg class="collect-icon" :class="{ 'collect-icon--active': isCollect }" width="24" height="24" viewBox="0 0 24 24"
                                :fill="isCollect ? 'currentColor' : 'none'" aria-hidden="true">
                                <path d="m12 3 2.78 5.64 6.22.9-4.5 4.39 1.06 6.2L12 17.2l-5.56 2.93 1.06-6.2L3 9.54l6.22-.9L12 3Z"
                                    stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" />
                            </svg>
                            <span class="collect-label">{{ $t("收藏") }}</span>
                        </div>
                    </div>
                </div>
            </Transition>
        </Teleport>
    </ClientOnly>
</template>
<script lang="ts" setup>
import RawImageDownloadButton from "~/components/product/RawImageDownloadButton.vue";
/**
 * @description 商品详情页悬浮购买栏（新版 UI）
 * - 固定悬浮在页面底部，与右侧商品信息栏(.right-wrap)对齐
 * - 复用 ProductBuy（加入购物车 / 立即购买 / 立即预约 / 立即兑换 / 立即预购）的购买逻辑
 * - 适配 O2O：透传 shopId、周期购(isCycleInfo)、预约商品(productType 5/6)
 * - 内置商品收藏功能
 */
import { ref, watch, computed, nextTick, onMounted, onBeforeUnmount } from "vue";
import { getCollectProduct } from "~/api/product/product";
import { updateCollectProduct, delCollectProduct } from "~/api/user/collectProduct";
import { useUserStore } from "~/store/user";
import { useThemeStore } from "~/store/theme";
import { useCommonStore } from "~/store/common";
import { isB2B, isOverseas, isS2b2c } from "@/utils/util";
import { message } from "ant-design-vue";
import type { ProductItem, ExchangeDetail } from "~/types/product/product.d";

const props = defineProps({
    product: {
        type: Object as PropType<ProductItem | Record<string, any>>,
        default: () => ({})
    },
    productType: {
        type: String,
        default: "product"
    },
    skuId: {
        type: Number,
        default: 0
    },
    number: {
        type: Number,
        default: 1
    },
    productStock: {
        type: Number,
        default: 0
    },
    extraAttrIds: {
        type: String,
        default: ""
    },
    isSeckill: {
        type: Number,
        default: 0
    },
    // 二期会员标签（D1096）：秒杀/限时折扣/拼团不在人群范围时置灰购买按钮
    memberRestricted: {
        type: Boolean,
        default: false
    },
    restrictTip: {
        type: String,
        default: ""
    },
    isPresale: {
        type: Boolean,
        default: false
    },
    presaleDetail: {
        type: Object,
        default: () => ({})
    },
    presalePriceValue: {
        type: [String, Number],
        default: ""
    },
    exchangeDetail: {
        type: Object as PropType<ExchangeDetail>,
        default: () => ({})
    },
    // O2O：门店ID
    shopId: {
        type: Number,
        default: null
    },
    // O2O：是否周期购
    isCycleInfo: {
        type: Boolean,
        default: false
    },
    // B2B 批量采购相关
    isBatch: {
        type: Boolean,
        default: false
    },
    // 已选规格数量列表：[{ skuId, num }]
    skuItem: {
        type: Array,
        default: () => []
    },
    // 拼团活动相关
    isGroupon: {
        type: Boolean,
        default: false
    },
    grouponType: {
        type: String,
        default: ""
    },
    isJoin: {
        type: [Number, Boolean],
        default: 1
    },
    // 已组装好的拼团下单参数：{ ...grouponDetail, groupNumKey, price, grouponPromotionRecordId }
    grouponBuyDetail: {
        type: Object,
        default: () => ({})
    },
    // 批发态总开关：true 时悬浮栏切换为批发操作胶囊（加购/立即购买/询价）
    isWholesale: {
        type: Boolean,
        default: false
    },
    // 批发态主按钮禁用（如商品下架）
    disabled: {
        type: Boolean,
        default: false
    },
    // 批发态加入购物车 loading
    wholesaleLoadingAdd: {
        type: Boolean,
        default: false
    },
    // 批发态立即购买 loading
    wholesaleLoadingBuy: {
        type: Boolean,
        default: false
    }
});

const emit = defineEmits(["enquiry", "add-to-cart", "buy-now"]);


const { t } = useI18n();
const themeStore = useThemeStore();
const commonStore = useCommonStore();
const visible = ref(true);

const showCollect = computed(() => props.productType === "product" || props.productType === "exchange");
// 立即询价：开启询价功能且为 B2B / 海外 / S2B2C 版本时展示
const showEnquiryBtn = computed(() => commonStore.isEnquiry === 1 && (isB2B() || isOverseas() || isS2b2c()));
const isOutOfStock = computed(() => {
    const stock = Number(props.productStock);
    return !Number.isFinite(stock) || stock <= 0;
});
// 预约商品（如：服务类、到店核销）缺货时仍可下单预约
const isReservation = computed(() => props.product?.productType === 5 || props.product?.productType === 6);

// 与商品详情页右侧栏(.right-wrap)对齐：读取其实际位置/宽度来定位悬浮面板
const panelRef = ref<HTMLElement | null>(null);
const panelStyle = ref<Record<string, string>>({});
const updatePanelPosition = () => {
    const el = document.querySelector(".right-wrap") as HTMLElement | null;
    if (!el) return;
    const rect = el.getBoundingClientRect();
    panelStyle.value = {
        left: `${rect.left - 16}px`,
        width: `${rect.width + 32}px`
    };
};

// 页脚进入视口后购买栏上移停靠，避免压住页面底部内容
const bottomOffset = ref(0);
const barStyle = computed(() => ({ bottom: `${bottomOffset.value}px` }));
const isDocked = computed(() => bottomOffset.value > 0);

const getDockElement = () => {
    const list = Array.from(document.querySelectorAll<HTMLElement>(".footer")).filter((el) => !el.closest(".el-overlay") && !el.closest(".el-dialog"));
    return list.length ? list[list.length - 1] : null;
};

const updateBottomOffset = () => {
    const el = getDockElement();
    if (!el) {
        bottomOffset.value = 0;
        return;
    }
    const overlap = window.innerHeight - el.getBoundingClientRect().top;
    if (overlap <= 0) {
        bottomOffset.value = 0;
        return;
    }
    // 页脚高于视口时限制上移幅度，保证购买栏始终可见
    const panelHeight = panelRef.value?.offsetHeight || 0;
    const max = Math.max(0, window.innerHeight - panelHeight - 24);
    bottomOffset.value = Math.min(overlap, max);
};

let rafId = 0;
const updateLayout = () => {
    if (rafId) return;
    rafId = window.requestAnimationFrame(() => {
        rafId = 0;
        updatePanelPosition();
        updateBottomOffset();
    });
};

let resizeObserver: ResizeObserver | null = null;
onMounted(async () => {
    await nextTick();
    updatePanelPosition();
    updateBottomOffset();
    window.addEventListener("resize", updateLayout);
    window.addEventListener("scroll", updateLayout, { passive: true });
    if (typeof ResizeObserver !== "undefined") {
        resizeObserver = new ResizeObserver(updateLayout);
        resizeObserver.observe(document.documentElement);
    }
});
onBeforeUnmount(() => {
    window.removeEventListener("resize", updateLayout);
    window.removeEventListener("scroll", updateLayout);
    resizeObserver?.disconnect();
    resizeObserver = null;
    if (rafId) window.cancelAnimationFrame(rafId);
});

// 收藏逻辑
const isCollect = ref(false);
const getCollect = async () => {
    const userStore = useUserStore();
    if (!userStore.isAuthenticated || !props.product?.productId) return;
    try {
        isCollect.value = await getCollectProduct({ id: props.product.productId });
    } catch (error: any) {
        // 静默失败，避免影响购买栏展示
    }
};

watch(
    () => props.product?.productId,
    (id) => {
        if (id) getCollect();
    },
    { immediate: true }
);

const toggleCollect = async () => {
    const userStore = useUserStore();
    if (!userStore.isAuthenticated) {
        const route = useRoute();
        navigateTo(`/member/login?returnUrl=${route.fullPath}`);
        return;
    }
    try {
        if (!isCollect.value) {
            await updateCollectProduct({ productId: props.product.productId });
        } else {
            await delCollectProduct({ id: props.product.productId });
        }
        const wasCollect = isCollect.value;
        await getCollect();
        message.success(wasCollect ? t("取消收藏成功") : t("收藏成功"));
    } catch (error: any) {
        message.error(error.message);
    }
};
</script>
<style lang="less" scoped>
.float-buy-bar {
    position: fixed;
    left: 0;
    right: 0;
    bottom: 0;
    z-index: 1000;
    pointer-events: none;

    &__panel {
        position: absolute;
        bottom: 12px;
        pointer-events: auto;
        box-sizing: border-box;
        display: flex;
        align-items: center;
        gap: 12px;
        padding: 16px;
        background: #fff;
        border-radius: 16px;
        box-shadow: 0 12px 36px 0 rgba(31, 31, 51, .12);

        &--docked {
            box-shadow: none;
        }

        // 未被 ProductBuy 包裹的禁用胶囊自带置灰；被包裹的交给 .buy.disabled，避免两层 opacity 相乘
        > .pill--disabled {
            opacity: 0.5;
        }
    }

    .pill {
        flex: 2;
        min-width: 0;
        display: flex;
        align-items: center;
        height: 46px;
        border-radius: 12px;
        overflow: hidden;
        cursor: pointer;
        font-size: 16px;
        font-weight: 600;
        color: var(--main-text);
        background: var(--main-bg);

        &:not(.split) {
            justify-content: center;
        }

        // split 态下两个子按钮铺满容器，容器底色仅在子按钮被禁用淡化时透出；
        // 必须与面板同为白色，否则半透明按钮会叠在主题色上，与风格1/3 的合成结果不一致
        &.split {
            background: #fff;
        }

        &.pill--buy {
            background: var(--main-bg);
            color: var(--main-text);
        }

        &.pill--disabled {
            cursor: not-allowed;
        }

        &__btn {
            flex: 1;
            display: flex;
            align-items: center;
            justify-content: center;
            height: 46px;
            transition: filter 0.2s;

            &:hover {
                filter: brightness(1.05);
            }

            &--cart {
                background: var(--vice-bg);
                color: var(--vice-text);
            }

            &--buy {
                background: var(--main-bg);
                color: var(--main-text);
            }

            &--disabled {
                // background: #d9d9d9;
                // color: #fff;
                cursor: not-allowed;

                &:hover {
                    filter: none;
                }
            }
        }

        // 直接置于胶囊内的禁用按钮没有 ProductBuy 的 .buy.disabled 外层，需自带置灰
        > .pill__btn--disabled {
            opacity: 0.5;
        }
    }

    // 立即询价：从胶囊中拆出的独立按钮（白底、浅灰边框与文字），宽度占胶囊区的三分之一
    &__inquiry {
        flex: 1;
        min-width: 0;
        display: flex;
        align-items: center;
        justify-content: center;
        height: 46px;
        border-radius: 12px;
        font-size: 16px;
        font-weight: 600;
        color: #606266;
        background: #fff;
        box-shadow: inset 0 0 0 1px #dcdfe6;
        cursor: pointer;
        transition: all 0.2s;

        &:hover {
            background: var(--vice-bg);
            color: var(--vice-text);
            box-shadow: inset 0 0 0 1px var(--vice-bg);
        }

        &--disabled {
            cursor: not-allowed;
            opacity: 0.5;

            &:hover {
                background: #fff;
                color: #606266;
                box-shadow: inset 0 0 0 1px #dcdfe6;
            }
        }
    }

    // ProductBuy 外层 .buy 容器铺满，保证整块可点击
    .pill-buy {
        flex: 1;
        display: flex;
    }
    :deep(.buy) {
        flex: 1;
        display: flex;
    }
    :deep(.buy.disabled) {
        cursor: not-allowed;
        opacity: 0.5;

        // 包裹层已整体淡化，内部按钮不得再响应 hover 变亮
        .pill:hover,
        .pill__btn:hover {
            filter: none;
        }
    }

    &__collect {
        flex-shrink: 0;
        width: 44px;
        display: flex;
        flex-direction: column;
        align-items: center;
        justify-content: center;
        gap: 2px;
        cursor: pointer;
        color: #666;
        font-size: 12px;
        font-weight: 400;
        line-height: 1;

        .collect-icon {
            display: block;
            flex-shrink: 0;
        }

        .collect-icon--active {
            color: var(--general);
        }

        .collect-label {
            line-height: 1;
            white-space: nowrap;
        }

        &:hover {
            color: var(--general);
        }
    }
}

.float-bar-slide-enter-active,
.float-bar-slide-leave-active {
    transition: transform 0.3s ease, opacity 0.3s ease;
}
.float-bar-slide-enter-from,
.float-bar-slide-leave-to {
    transform: translateY(160px);
    opacity: 0;
}
</style>
