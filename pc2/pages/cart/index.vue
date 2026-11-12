<template>
    <div class="cart">
        <CommonPageHeader></CommonPageHeader>

        <CommonHeader title="购物车"></CommonHeader>

        <template v-if="!loading">
            <CommonPageContainer>
                <div data-panel-rev="MjI4MmlqZXI" class="cart-title">
                    <div class="r_wheit justify-center" v-if="!loading && cartList.length === 0">
                        <div class="no_goods">
                            <div class="flex">
                                <SvgIcon name="shop"></SvgIcon>
                                <div class="strong">{{ $t("还没有商品？") }}</div>
                            </div>
                            <div class="left main">
                                <NuxtLink to="/">
                                    <el-button size="large" class="go_buy" :title="$t('马上挑选商品')">{{ $t("去购物") }}</el-button>
                                </NuxtLink>
                            </div>
                        </div>
                    </div>
                    <div v-else class="cart-left-col">
                        <div class="cart-head">
                            <div class="cartHeadTitle">
                                {{ $t("购物车") }}
                            </div>
                            <div class="product-list-cart flex">
                                <div class="item col-1 center">
                                    <el-checkbox class="block-checkbox" v-model="allChecked" @change="onCheckAll"></el-checkbox>
                                </div>
                                <div class="item allItem" style="justify-content: left; color: #1a1a1a">{{ $t("全选") }}</div>
                                <div class="cart-batch-actions flex align-center">
                                    <div class="batch-btn" @click="collectCheckedCartItem">{{ $t("移入收藏") }}</div>
                                    <div class="batch-btn" @click="removeCheckedCartItem">{{ $t("删除") }}</div>
                                </div>
                            </div>
                        </div>
                        <div v-if="cartList" class="cart_table" v-for="(item, index) in cartList" :key="index">
                            <div class="cart_store_title flex">
                                <div class="item col-1 height-1 center">
                                    <el-checkbox
                                        class="block-checkbox"
                                        v-model="item.isChecked"
                                        :disabled="isShopAllDisabled(item)"
                                        @change="onCheckAllItem(item)"
                                    ></el-checkbox>
                                </div>
                                <div class="item flex align-center">
                                    <div class="tit" v-if="item.shopId == 0">{{ $t("官方自营") }}</div>
                                    <div class="tit" v-else-if="isMerchant() || isStore()">{{ item.shopTitle }}</div>
                                </div>
                            </div>
                            <div class="cart_list_wrap">
                                <div
                                    class="product-list-cart"
                                    v-for="product in item.carts"
                                    :class="isDisabledData(product) ? 'disabled-item' : ''"
                                    :data-wholesale-cart-id="(product as any).wholesaleScope ? product.cartId : undefined"
                                >
                                    <div class="product-item flex">
                                        <div class="item col-1 height-1 center flex align-center justify-center">
                                            <el-checkbox
                                                class="block-checkbox"
                                                v-model="product.isChecked"
                                                @change="onChangeCheck()"
                                                :disabled="isDisabledData(product)"
                                            >
                                            </el-checkbox>
                                        </div>
                                        <div
                                            class="item col-4 flex"
                                            :class="{ 'cart-invalid-product-item': isSkuDisabledData(product) || isOffShelfDisabledData(product) }"
                                        >
                                            <div class="pic">
                                                <a
                                                    v-if="!isProductUnavailable(product)"
                                                    target="_blank"
                                                    :href="
                                                        urlFormat(buildCartProductRoute(product))
                                                    "
                                                >
                                                    <img :src="productImage(product)" :alt="product.productName || ''" />
                                                    <img
                                                        v-if="product.productId && styleMap[product.productId]?.borderImage"
                                                        class="product-style-border"
                                                        :src="styleMap[product.productId].borderImage || undefined"
                                                        alt=""
                                                    />
                                                </a>
                                                <div v-else>
                                                    <img :src="productImage(product)" :alt="product.productName || ''" />
                                                    <img
                                                        v-if="product.productId && styleMap[product.productId]?.borderImage"
                                                        class="product-style-border"
                                                        :src="styleMap[product.productId].borderImage || undefined"
                                                        alt=""
                                                    />
                                                </div>
                                                <div class="disabled" v-if="isProductUnavailable(product) || product.stock == 0">
                                                    <div v-if="isProductUnavailable(product)">{{ $t("已下架") }}</div>
                                                    <div v-else-if="product.stock == 0">{{ $t("已售罄") }}</div>
                                                </div>
                                            </div>
                                            <div class="name pt5">
                                                <a
                                                    class="name_pctyle"
                                                    v-if="!isProductUnavailable(product)"
                                                    target="_blank"
                                                    :href="
                                                        urlFormat(buildCartProductRoute(product))
                                                    "
                                                >
                                                    {{ product.productName }}
                                                </a>
                                                <p v-else>
                                                    {{ product.productName }}
                                                </p>
                                                <div class="info">
                                                    <span v-for="(sku, _index) in product.skuData">
                                                        {{ _index !== 0 ? " , " : "" }}{{ sku.name }}:{{ sku.value }}</span
                                                    >
                                                </div>
                                                <template v-if="product.extraSkuData && product.extraSkuData.length">
                                                    <div class="extraskudata">
                                                        <template v-for="attr in product.extraSkuData" :key="attr">
                                                            <div class="extraskudata-item">
                                                                <div class="title">{{ attr.attrName }}</div>
                                                                <div class="line-box">
                                                                    <div class="line"></div>
                                                                </div>
                                                                <div class="content">
                                                                    <span class="content-text">{{ attr.attrValue }}</span>
                                                                    <FormatPrice v-model="attr.attrPrice" :showText="false"> </FormatPrice>
                                                                    <span class="num">x{{ product.quantity }}</span>
                                                                </div>
                                                            </div>
                                                        </template>
                                                    </div>
                                                </template>
                                                <template v-if="product.activityInfo && !isSkuDisabledData(product) && !isOffShelfDisabledData(product)">
                                                    <div class="promotion-box">
                                                        <div
                                                            class="promotion-box-item"
                                                            v-for="subItem in formatActivityList(product.activityInfo, (item as any).usedPromotions)"
                                                            :key="subItem.promotionId"
                                                        >
                                                            <div class="promotion" v-if="subItem.type != 2">
                                                                <div class="ticket-content">
                                                                    {{ activityTextMap[subItem.type] ? $t(activityTextMap[subItem.type]) : "" }}
                                                                </div>
                                                            </div>

                                                            <div class="promotion" v-if="subItem.type == 2">
                                                                <div class="ticket-type">{{ $t("券") }}<span class="ticket-line"></span></div>
                                                                <div class="ticket-content">{{ subItem.data.promotionDesc }}</div>
                                                            </div>
                                                        </div>
                                                    </div>
                                                </template>
                                                <div
                                                    class="item col-2"
                                                    :class="{ 'cart-invalid-item': isSkuDisabledData(product) || isOffShelfDisabledData(product) }"
                                                >
                                                    <div class="pt5 flex margin-top">
                                                        <InputNumber
                                                            v-if="isRetailLine(product) && !isSkuDisabledData(product) && !isOffShelfDisabledData(product)"
                                                            :limitNumber="(product as any).limitNumber"
                                                            :max="product.stock"
                                                            v-model="product.quantity"
                                                            @change="handleUpdateCartItem(product)"
                                                            :disabled="isDisabledData(product)"
                                                            @removeCartItem="removeCartItem(product.cartId)"
                                                        >
                                                        </InputNumber>
                                                        <span v-if="isSkuDisabledData(product)" class="cart-invalid-text">
                                                            {{ $t("请重新选择规格") }}
                                                        </span>
                                                        <span v-else-if="isOffShelfDisabledData(product)" class="cart-invalid-text">
                                                            {{ $t("商品已下架") }}
                                                        </span>
                                                    </div>
                                                </div>
                                            </div>
                                        </div>
                                        <!-- <div class="item col-2 center">
                                        <div class="price pt5 center-price priceForm">
                                            <FormatPrice v-model="product.price" :showText="false"></FormatPrice>
                                        </div>
                                    </div> -->
                                        <!-- <div class="item col-2 center">
                                        <div class="pt5 flex justify-center">
                                            <InputNumber
                                                :max="product.stock"
                                                v-model="product.quantity"
                                                @change="handleUpdateCartItem(product)"
                                                :disabled="isDisabledData(product)"
                                                @removeCartItem="removeCartItem(product.cartId)"
                                            >
                                            </InputNumber>
                                        </div>
                                    </div> -->
                                        <div class="item col-2 center" style="flex: 1; justify-content: end; margin-right: 20px; align-items: start">
                                            <div v-if="!isSkuDisabledData(product) && !isOffShelfDisabledData(product)" class="flex-f">
                                                <div class="money pt5 priceFormOurty discount-price">
                                                    <FormatPrice v-model="product.price" :showText="false"></FormatPrice>
                                                </div>
                                                <div class="money pt5 priceFormQurty orgin-price">
                                                    <FormatPrice v-model="(product as any).productPrice" :showText="false"></FormatPrice>
                                                    <div class="discount-txt">
                                                        {{ $t("优惠{0}%", [getDiscount(Number(product.price), Number((product as any).productPrice))]) }}
                                                    </div>
                                                </div>
                                            </div>
                                            <span v-else class="cart-invalid-text">{{ $t("暂不支持购买") }}</span>
                                        </div>
                                        <!-- <div class="item col-2 center">
                                        <div>
                                            <a @click="removeCartItem(product.cartId)" style="cursor: pointer">
                                                <i class="iconfont-pc icon-shanchu"></i>
                                            </a>
                                        </div>
                                    </div> -->
                                    </div>
                                    <div class="cart-line"></div>
                                </div>
                            </div>
                            <template v-if="item.gift.length > 0">
                                <div class="gift-box">
                                    <div class="gift-title">{{ $t("赠品") }}</div>
                                    <div class="gift-content">
                                        <template v-for="subItem in item.gift" :key="subItem.giftId">
                                            <NuxtLink
                                                target="_blank"
                                                :to="
                                                    urlFormat({
                                                        path: 'product',
                                                        seoLink: subItem.seoLink,
                                                        id: subItem.productId,
                                                        sn: subItem.productSn
                                                    })
                                                "
                                            >
                                                <div class="gift-content-item">
                                                    <div class="gift-item-left">
                                                        <el-image class="gift-img" lazy :src="productImage(subItem)" :alt="subItem.productName || ''">
                                                        </el-image>
                                                    </div>
                                                    <div class="gift-item-right">
                                                        <div class="gift-item-name">{{ subItem.productName }}</div>
                                                        <template v-if="subItem.skuData.length > 0">
                                                            <div class="sku-card">
                                                                <template v-for="(skuItem, skuIndex) in subItem.skuData" :key="skuIndex">
                                                                    <div class="sku-item">{{ skuItem.value }}</div>
                                                                </template>
                                                            </div>
                                                        </template>
                                                        <div class="gift-item-num">x{{ subItem.num }}</div>
                                                    </div>
                                                </div>
                                            </NuxtLink>
                                        </template>
                                    </div>
                                </div>
                            </template>
                        </div>
                        <div class="table-bar-total">
                            <div class="table-bar-total_l flex align-center">
                                <div class="item col-1 height-1 center flex align-center justify-center">
                                    <el-checkbox class="block-checkbox" v-model="allChecked" @change="onCheckAll"></el-checkbox>
                                </div>
                                <div class="pay_txt_row flex">
                                    <span class="all">
                                        {{ $t("全选") }}
                                    </span>
                                    <div class="flex total-num">
                                        <span> {{ $t("共") }} </span>
                                        <span class="font-color font-num">
                                            {{ total.totalCount }}
                                        </span>
                                        <span>
                                            {{ $t("件") }}
                                        </span>
                                    </div>
                                    <div>
                                        <a @click="onClearCart">{{ $t("清空购物车") }}</a>
                                    </div>
                                </div>
                            </div>
                        </div>
                        <!-- <div v-if="cartList.length != 0" class="emptied" @click="onClearCart">清空购物车</div> -->
                    </div>

                    <div class="cart_total">
                        <div :class="{ pay_tools_bar: true, tools_bar_bottom: isFixed }">
                            <div class="pay_tools_inner">
                                <div class="pay_tools_r">
                                    <div class="pay_tools_total_box" :class="{ 'is-expanded': isSettleExpanded }">
                                        <!-- 1. 头部展示结算明细标题和副标题 -->
                                        <div class="settle-header-container">
                                            <div class="settle-head flex justify-between align-center">
                                                <span class="settle-title">{{ $t("结算明细") }}</span>
                                                <span class="settle-sub">{{ $t("实际优惠金额以下单页为准") }}</span>
                                            </div>
                                        </div>

                                        <!-- 2. 中间展示已选择商品和价格明细（可滚动区域） -->
                                        <div class="settle-panel-container">
                                            <template v-if="!loading && total.checkedCount === 0">
                                                <div class="pay_tools_r flex justify-center" style="padding: 40px 0">
                                                    <div class="no_goods">
                                                        <div class="flex">
                                                            <SvgIcon name="shop"></SvgIcon>
                                                            <div class="strong">{{ $t("没有待结算的商品") }}</div>
                                                        </div>
                                                    </div>
                                                </div>
                                            </template>
                                            <template v-else>
                                                <div>
                                                    <!-- 已选择结算商品缩略图（超过 2 行折叠，settleFoldLimit 控制） -->
                                                    <div class="settle-goods-wrapper">
                                                        <div class="settle-goods" :class="{ 'is-fold': !showAllProducts }">
                                                            <div class="settle-goods-item" v-for="p in checkedProducts" :key="p.cartId">
                                                                <div class="settle-goods-img-mask"></div>
                                                                <div class="settle-goods-uncheck" @click.stop="onUncheckSettleProduct(p)">
                                                                    {{ $t("取消选择") }}
                                                                </div>
                                                                <el-image fit="contain" :src="productImage(p)" :alt="p.productName || ''"></el-image>
                                                            </div>
                                                            <!-- 补空节点，保持每行 4 个元素，防止 space-between 导致最后一行不足 4 个时两端对齐 -->
                                                            <div
                                                                class="settle-goods-item empty"
                                                                v-for="n in (4 - (checkedProducts.length % 4)) % 4"
                                                                :key="'empty-' + n"
                                                            ></div>
                                                        </div>
                                                        <div
                                                            class="settle-goods-toggle"
                                                            v-if="checkedProducts.length > settleFoldLimit"
                                                            @click="showAllProducts = !showAllProducts"
                                                        >
                                                            <span>{{ showAllProducts ? $t("收起") : $t("显示全部商品") }}</span>
                                                            <span class="arrow iconfont-pc icon-xiajiantou" :class="{ up: showAllProducts }"></span>
                                                        </div>
                                                    </div>
                                                    <div class="settle-price-wrapper">
                                                        <div class="settle-line-item">
                                                            <div class="settle-line">
                                                                <span class="lbl strong">{{ $t("商品总价") }}</span>
                                                                <span class="val"
                                                                    ><FormatPrice
                                                                        :showText="false"
                                                                        v-model="total.productAmount"
                                                                        :currency-style="{ fontWeight: 600, fontSize: '14px', lineHeight: '18px' }"
                                                                        :font-style="{ fontWeight: 600, fontSize: '16px', lineHeight: '20px' }"
                                                                    ></FormatPrice
                                                                ></span>
                                                            </div>
                                                        </div>
                                                    </div>
                                                </div>

                                                <div class="settle-lines">
                                                    <div class="settle-line-item" v-if="total.discountDiscountAmount > 0">
                                                        <div class="settle-line">
                                                            <span class="lbl">{{ $t("活动优惠") }}</span>
                                                            <span class="val discount-val flex align-center">
                                                                <span class="settle-symbol">-</span>
                                                                <FormatPrice :showText="false" v-model="total.discountDiscountAmount"></FormatPrice>
                                                            </span>
                                                        </div>
                                                    </div>
                                                    <div class="settle-line-item" v-if="total.discountCouponAmount > 0">
                                                        <div class="settle-line">
                                                            <span class="lbl">{{ $t("优惠券") }}</span>
                                                            <span class="val discount-val flex align-center">
                                                                <span class="settle-symbol">-</span>
                                                                <FormatPrice :showText="false" v-model="total.discountCouponAmount"></FormatPrice>
                                                            </span>
                                                        </div>
                                                    </div>
                                                    <div class="settle-line-item" v-if="total.serviceFee && total.serviceFee > 0">
                                                        <div class="settle-line">
                                                            <span class="lbl">{{ $t("附加费用") }}</span>
                                                            <span class="val flex align-center">
                                                                <span class="settle-symbol">+</span>
                                                                <FormatPrice :showText="false" v-model="total.serviceFee"></FormatPrice>
                                                            </span>
                                                        </div>
                                                    </div>
                                                </div>
                                                <div class="settle-summary-row flex justify-end align-center" v-if="!loading && total.checkedCount > 0">
                                                    <div class="save-total">
                                                        <span class="lbl">{{ $t("优惠共减") }}</span>
                                                        <FormatPrice
                                                            :showText="false"
                                                            v-model="total.discounts"
                                                            :currency-style="{ fontWeight: 600, fontSize: '14px' }"
                                                            :font-style="{ fontWeight: 600, fontSize: '16px' }"
                                                        ></FormatPrice>
                                                    </div>
                                                    <span class="divider-line"></span>
                                                    <div class="sub-total">
                                                        <span class="lbl">{{ $t("合计") }}</span>
                                                        <FormatPrice
                                                            :showText="false"
                                                            v-model="total.discountAfter"
                                                            :currency-style="{ fontWeight: 600, fontSize: '14px' }"
                                                            :font-style="{ fontWeight: 600, fontSize: '16px' }"
                                                        ></FormatPrice>
                                                    </div>
                                                </div>
                                            </template>
                                        </div>

                                        <!-- 3. 底部展示合计以及购买按钮 -->
                                        <div class="settle-bottom-container">
                                            <div class="settle-bottom-inner">
                                                <div class="settle-bottom-left">
                                                    <div class="settle-final flex justify-between align-end" v-if="!loading">
                                                        <span class="final-label">{{ $t("合计") }}:</span>
                                                        <div class="final-right">
                                                            <FormatPrice
                                                                class="final-price"
                                                                :showText="false"
                                                                v-model="total.discountAfter"
                                                                :currency-style="{
                                                                    fontWeight: 600,
                                                                    fontSize: '16px',
                                                                    color: 'var(--general)',
                                                                    lineHeight: '20px'
                                                                }"
                                                                :font-style="{ fontWeight: 600, fontSize: '24px', color: 'var(--general)', lineHeight: '26px' }"
                                                            ></FormatPrice>
                                                        </div>
                                                    </div>
                                                    <!-- 优惠共减 & 优惠明细切换 (仅在小屏幕下显示/起作用) -->
                                                    <div
                                                        class="settle-discount-toggle"
                                                        v-if="!loading && total.checkedCount > 0"
                                                        @click="isSettleExpanded = !isSettleExpanded"
                                                    >
                                                        <span class="discount-lbl">{{ $t("优惠共减") }}</span>
                                                        <FormatPrice
                                                            class="discount-val"
                                                            :showText="false"
                                                            v-model="total.discounts"
                                                            :currency-style="{ fontWeight: 500, fontSize: '12px', color: 'var(--general)' }"
                                                            :font-style="{ fontWeight: 500, fontSize: '12px', color: 'var(--general)' }"
                                                        ></FormatPrice>
                                                        <span class="toggle-link">
                                                            <span>{{ $t("优惠明细") }}</span>
                                                            <span class="arrow iconfont-pc icon-xiajiantou" :class="{ up: isSettleExpanded }"></span>
                                                        </span>
                                                    </div>
                                                </div>
                                                <div class="settle-bottom-right">
                                                    <TigButton
                                                        theme
                                                        :disabled="!total.checkedCount"
                                                        class="buyBtnOne"
                                                        type="primary"
                                                        @click="total.checkedCount && toCheck()"
                                                        >{{ $t("结算") }}({{ total.checkedCount ? total.checkedCount : 0 }})</TigButton
                                                    >
                                                </div>
                                            </div>
                                        </div>
                                    </div>
                                </div>
                            </div>
                        </div>
                        <!-- </div> -->
                    </div>
                </div>
                <ProdSwiper titleName="精选好物" @callback="updateCart"></ProdSwiper>
            </CommonPageContainer>
        </template>
        <template v-if="loading">
            <div>
                <div style="height: 600px; width: 100%"></div>
            </div>
        </template>
        <CommonPageFooter></CommonPageFooter>
    </div>
</template>

<script lang="ts" setup>
import { isProductUnavailable } from "@/utils/productSale";
import { navigateTo } from "nuxt/app";
import InputNumber from "./src/InputNumber.vue";
import ProdSwiper from "./src/ProdSwiper.vue";
import { message, Modal } from "ant-design-vue";
import { getCart, updateCartCheck, updateCartItemData, removeCartItemData, clearCart } from "@/api/cart";
import { updateCollectProduct } from "@/api/user/collectProduct";
import type { CartFilterState, TotalObj, CartItmes } from "@/types/cart/cart.d";
import { urlFormat } from "@/utils/format";
import { ElLoading } from "element-plus";
import { isMerchant, isStore } from "@/utils/util";
import { useCommonStore } from "@/store/common";
import { asyncGetCartCount } from "~/api/cart";
import { useCartStore } from "~/store/cart";
import { buildCartProductRoute, mapCartLineShopIds } from "@/utils/cartScope";
import { useProductStyle } from "~/hooks";

definePageMeta({
    // 购物车仅跨境版强制登录；o2o/标准版游客可访问。中间件内再经 isOverseas() 运行时二次守卫（融合 dev 未裁时也只对跨境生效）。
    middleware: "overseas-cart-auth"
});

// 样式版本切换预留位（仿商品详情 Product.vue 的 uiVersion/isNewUi）：
// 当前只出跨境样式基底、不做实际切换；将来接 getCartStyle() 或 isOverseas() 时在此落分支。
const commonStore = useCommonStore();

const { t } = useI18n();
const cartStore: any = useCartStore();

/**
 * 是否按零售计步器渲染该行
 * - 批发行使用批发计步器，其余行使用零售计步器
 */
const isRetailLine = (product: CartItmes): boolean => {
    return true;
};
const loading = ref(true);
const cartList = ref<CartFilterState[]>([]);
const cartProducts = computed(() => cartList.value.flatMap((item) => item.carts ?? []));

// 结算明细：已勾选商品（缩略图展示用）+ 折叠控制（每行 4 个，超过 2 行折叠）
const checkedProducts = computed(() => cartProducts.value.filter((p) => p.isChecked));
const showAllProducts = ref(false);
const isSettleExpanded = ref(false);
const settleFoldLimit = 8;
const { styleMap } = useProductStyle(cartProducts);
const total = ref<TotalObj>({} as TotalObj);
const cartLoading = ref();
const step = ref<number>(0);
onBeforeMount(() => {
    updateCart();
});

/**
 * @description 判断购物车商品是否处于禁用状态（下架/无库存/禁用）
 * @param product 购物车商品项
 * @returns 是否禁用
 */
/**
 * @description 判断店铺内是否所有商品都已下架/售罄/禁用
 */
const isShopAllDisabled = (item: CartFilterState) => {
    return item.carts.length > 0 && item.carts.every((product) => isDisabledData(product));
};

const isDisabledData = (product: CartItmes) => {
    if (isProductUnavailable(product) || product.stock == 0 || product.isDisabled == true) {
        return true;
    }
    return false;
};

const isSkuDisabledData = (product: CartItmes) => product.sku?.isEnabled === 0;
const isOffShelfDisabledData = (product: CartItmes) => isProductUnavailable(product);

/**
 * @description 刷新购物车数据，获取最新商品列表和价格统计
 * @returns 无返回值，更新cartList和total状态
 */
const updateCart = async () => {
    try {
        cartLoading.value = ElLoading.service({
            lock: false,
            text: t("请求加载中"),
            customClass: "tig-normal-loading"
        });
        const result = await getCart();
        cartList.value = mapCartLineShopIds(result.cartList);
        total.value = result.total;
        updateCheckbox();
        _getCartCount();
    } catch (error) {
    } finally {
        loading.value = false;
        cartLoading.value.close();
    }
};

/**
 * @description 获取购物车商品数量并更新全局状态（用于头部购物车图标显示）
 */
const _getCartCount = async () => {
    try {
        const result = await asyncGetCartCount();
        cartStore.cartCount = result;
    } catch (error) {}
};

/**
 * @description 更新购物车商品勾选状态到服务端
 * @returns 无返回值，更新后刷新购物车
 */
const updateCheckData = async () => {
    try {
        const checkData = <{ cartId: number; isChecked: number }[]>[];
        cartList.value.forEach((item) => {
            item.carts.forEach((product) => {
                checkData.push({
                    cartId: product.cartId,
                    isChecked: product.isChecked && !isProductUnavailable(product) && product.isDisabled === false ? 1 : 0
                });
            });
        });
        const result = await updateCartCheck({ data: checkData });
        updateCart();
    } catch (error) {
    } finally {
    }
};

const handleUpdateCartItem = (product: CartItmes) => {
    startDelayTimer(product);
};
let delayTimer: any; // 延时定时器
/**
 * @description 防抖更新购物车商品数量，避免频繁请求
 * @param product 购物车商品项
 */
const startDelayTimer = (product: CartItmes) => {
    if (delayTimer) {
        clearTimeout(delayTimer);
    }
    delayTimer = setTimeout(() => {
        updateCartItem(product);
    }, 300);
};
/**
 * @description 更新购物车单个商品数量
 * @param product 购物车商品项，包含cartId和quantity
 */
const updateCartItem = async (product: CartItmes) => {
    try {
        await updateCartItemData(product.cartId, {
            quantity: product.quantity
        });
        await updateCart();
    } catch (error: any) {
        console.log(error);
        message.error(error.message);
    } finally {
    }
};

/**
 * @description 删除单个购物车商品，需用户确认
 * @param cartId 购物车项ID
 */
const removeCartItem = (cartId: number) => {
    Modal.confirm({
        title: t("确认要删除该商品吗"),
        cancelText: t("取消"),
        okText: t("确定"),
        maskClosable: true,
        centered: true,
        onOk: async () => {
            try {
                const result = await removeCartItemData([cartId]);
                updateCart();
            } catch (error) {
            } finally {
            }
        }
    });
    return;
};

/**
 * @description 删除所有已勾选的购物车商品
 */
const removeCheckedCartItem = () => {
    let index: number = 0;
    cartList.value.forEach((item) => {
        item.carts.forEach((product) => {
            if (product.isChecked == true) {
                index += 1;
            }
        });
    });
    if (index != 0) {
        Modal.confirm({
            title: t("确认要删除指定的商品吗"),
            cancelText: t("取消"),
            okText: t("确定"),
            maskClosable: true,
            centered: true,
            onOk: async () => {
                try {
                    const cartIds = <number[]>[];
                    cartList.value.forEach((item) => {
                        item.carts.forEach((product) => {
                            if (product.isChecked) cartIds.push(product.cartId);
                        });
                    });
                    const result = await removeCartItemData(cartIds);
                    await updateCart();
                } catch (error) {
                } finally {
                }
            }
        });
    } else {
        message.warning(t("请至少选择一个宝贝"));
    }
};

// 移入收藏：对已勾选商品逐个收藏后从购物车移除（仅操作勾选项，空选提示）
const collectCheckedCartItem = () => {
    const checked = cartProducts.value.filter((p) => p.isChecked);
    if (checked.length === 0) {
        return message.warning(t("请至少选择一个宝贝"));
    }
    Modal.confirm({
        title: t("确认将选中的商品移入收藏吗？"),
        cancelText: t("取消"),
        okText: t("确定"),
        maskClosable: true,
        centered: true,
        onOk: async () => {
            try {
                // 已收藏商品调用收藏接口会失败，但不影响从购物车移除
                await Promise.allSettled(checked.map((p) => updateCollectProduct({ productId: p.productId })));
                await removeCartItemData(checked.map((p) => p.cartId));
                await updateCart();
            } catch (error) {}
        }
    });
};

const onClearCart = () => {
    Modal.confirm({
        title: t("确认要清空购物车吗？"),
        cancelText: t("取消"),
        okText: t("确定"),
        maskClosable: true,
        centered: true,
        onOk: async () => {
            try {
                const result = await clearCart();
                await updateCart();
            } catch (error) {
            } finally {
            }
        }
    });
};

const onChangeCheck = () => {
    updateCheckData();
};

const onUncheckSettleProduct = (product: CartItmes) => {
    product.isChecked = false;
    updateCheckbox();
    updateCheckData();
};

const onCheckAll = () => {
    cartList.value.forEach((item) => {
        item.carts.forEach((product) => {
            product.isChecked = allChecked.value == true && product.isDisabled === false ? true : false;
        });
    });
    updateCheckbox();
    updateCheckData();
};

const onCheckAllItem = (item: CartFilterState) => {
    item.carts.forEach((product) => {
        product.isChecked = item.isChecked == true && !isProductUnavailable(product) && !product.isDisabled;
    });
    updateCheckbox();
    updateCheckData();
};
// 更新checkbox
const allChecked = ref(false);
const updateCheckbox = () => {
    cartList.value.forEach((item) => {
        const availableProducts = item.carts.filter((product) => !product.isDisabled);
        item.isChecked = availableProducts.length > 0 && availableProducts.every((product) => product.isChecked === true && !product.isDisabled);
    });
    const checkableItems = cartList.value.filter((item) => item.carts.some((product) => !product.isDisabled));
    allChecked.value = checkableItems.length > 0 && checkableItems.every((item) => item.isChecked === true);
};

const isFixed = ref(false);

interface ActivityTextMap {
    [key: number]: string;
}
const activityTextMap: ActivityTextMap = {
    1: "秒杀价",
    2: "券",
    3: "满减",
    4: "折扣",
    5: "满赠",
    6: "限时折扣",
    7: "拼团"
};
const formatActivityList = (data: any, usedPromotions: any = []) => {
    const activityList: any[] = [];
    const list = Array.isArray(data) ? data : [];
    const usedList = Array.isArray(usedPromotions) ? usedPromotions : [];
    const couponList = list.filter((item: any) => item.type === 2);
    const promotionList = list.filter((item: any) => item.type !== 2);
    const usedCouponList = usedList.filter((item: any) => item.type === 2);
    if (promotionList.length > 0) activityList.push(promotionList[0]);
    if (couponList.length > 0) {
        const usedKeySet = new Set(usedCouponList.map((x: any) => x?.promotionId).filter((x: any) => x !== undefined && x !== null));
        const usedCoupon = couponList.find((c: any) => usedKeySet.has(c?.promotionId));
        activityList.push(usedCoupon || couponList[0]);
    }

    return activityList;
};

const getDiscount = (currentPrice: number, originalPrice: number) => {
    if (originalPrice === 0 || currentPrice >= originalPrice) {
        return 0;
    }
    const discountPercentage = ((originalPrice - currentPrice) / originalPrice) * 100;

    return Math.floor(discountPercentage * 10) / 10;
};

const showDiscountInfo = ref(false);

// onMounted(() => {
//     window.addEventListener("scroll", updateAttrActionFixed);
//     updateAttrActionFixed();
// });

// onUnmounted(() => {
//     window.removeEventListener("scroll", updateAttrActionFixed);
// });

const toCheck = async () => {
    if (!(await refreshSaleBeforeCheckout())) return;
    step.value = 1;
    if (commonStore.closeOrder === 1) return message.error(t("商城已关闭下单"));
    navigateTo("/order/check/");
};

let saleCheckoutPending = false;
const refreshSaleBeforeCheckout = async () => {
    if (saleCheckoutPending) return false;
    saleCheckoutPending = true;
    const selectedIds = new Set(cartList.value.flatMap((group: any) => group.carts ?? []).filter((line: any) => line.isChecked).map((line: any) => line.cartId));
    try {
        const fresh = await getCart();
        cartList.value = fresh.cartList;
        total.value = fresh.total;
        updateCheckbox();
        const lines = fresh.cartList.flatMap((group: any) => group.carts ?? []);
        const blocked = lines.some((line: any) => selectedIds.has(line.cartId) && isProductUnavailable(line));
        if (blocked) {
            await updateCartCheck({ data: lines.map((line: any) => ({ cartId: line.cartId, isChecked: line.isChecked && !isProductUnavailable(line) ? 1 : 0 })) });
            message.error(t("商品已下架"));
            return false;
        }
        return lines.some((line: any) => line.isChecked && !isProductUnavailable(line));
    } catch (error: any) {
        message.error(error.message);
        return false;
    } finally {
        saleCheckoutPending = false;
    }
};

</script>

<style lang="less" scoped>
// 购物车宽度跟随全站 --pc-home-* 分流（app.vue 按首页新旧版统一赋值），不再本地硬设
.extraskudata {
    color: #999;

    .extraskudata-item {
        display: flex;

        .title {
            padding-right: 10px;
        }

        .content-text {
            padding-right: 5px;
        }

        .num {
            padding-left: 5px;
        }
    }
}

.discount {
    margin-top: 16px;
}

.discount-title {
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 14px;
}

.discount-content {
    font-size: 13px;

    .discount-item-box {
        display: flex;
        align-items: center;
        justify-content: space-between;
        // padding: 10px 0;
        color: #1a1a1a;

        .discounts-value {
            font-weight: bold;
        }

        .item-value {
            font-size: 14px;
        }

        .sub-item-box {
            display: flex;
            align-items: center;
            justify-content: space-between;
            padding: 10px 0 7px;
            font-size: 12px;
        }

        &:last-child {
            padding-top: 0;
        }
    }

    hr {
        height: 1px;
        color: #f5f6fa;
        border-width: 0;
        border-style: solid;
        border-color: rgba(0, 0, 0, 0.12);
        border-bottom-width: thin;
        margin: 0;
        background: transparent;
    }

    .discount-detail {
        display: flex;
        align-items: center;
        justify-content: space-between;
        margin-bottom: 10px;
        font-size: 12px;
        color: #969799;
        letter-spacing: 0;
    }
}

.discount-desc {
    margin-top: 16px;
    margin-bottom: 16px;
    font-size: 14px;
    line-height: 16px;
    font-weight: 400;
    text-align: left;
    color: #888b94;
}

.gift-box {
    font-size: 14px;
    position: relative;
    margin: 0 20px;
    padding-top: 8px;
    margin-top: 5px;

    &::after {
        content: " ";
        position: absolute;
        pointer-events: none;
        box-sizing: border-box;
        -webkit-transform-origin: center;
        transform-origin: center;
        top: 0;
        left: 0;
        width: 100%;
        bottom: auto;
        -webkit-transform: scaleY(0.5);
        transform: scaleY(0.5);
        border-top: 1px solid #ebedf0;
    }

    .gift-content {
        // display: grid;
        grid-template-columns: repeat(3, 1fr);
        grid-gap: 30px;
        padding: 10px 0;
    }

    .gift-content-item {
        display: flex;
        font-size: 12px;
        padding: 10px;
        padding-bottom: 8px;
        position: relative;
        background-color: var(--tag-bg);
        border-radius: 10px;

        .gift-item-left {
            width: 60px;
            height: 60px;
            border-radius: 4px;
            overflow: hidden;
            margin-right: 10px;
        }

        .gift-item-right {
            width: calc(100% - 60px);

            .gift-item-name {
                width: 100%;
                word-break: break-all;
                display: -webkit-box;
                -webkit-line-clamp: 2;
                -webkit-box-orient: vertical;
                overflow: hidden;
                color: var(--tag-text);
            }
        }

        .gift-item-num {
            font-size: 12px;
            color: #969799;
            position: absolute;
            bottom: 8px;
            right: 10px;
        }
    }

    .sku-card {
        display: inline-template;
        color: #969799;
        margin: 5px 0;

        .sku-item {
            display: inline-template;
            max-width: 130rpx;
        }

        .sku-item::before {
            content: ",";
            padding-left: 5rpx;
        }

        .sku-item:first-child::before {
            content: "";
        }
    }
}

.promotion-box {
    display: flex;
    align-items: center;
    padding-top: 8px;
    column-gap: 10px;

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
        align-items: stretch;

        .ticket-type {
            padding: 0 6px;
            max-width: 100px;
            position: relative;
            display: flex;
            align-items: center;

            .ticket-line {
                position: absolute;
                display: inline-template;
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
            max-width: 250px;
            overflow: hidden;
            text-overflow: ellipsis;
            white-space: nowrap;
            display: flex;
            align-items: center;
        }
    }
}

.margin-top {
    margin-top: 8px;
}

.col-1 {
    width: 60px;
}

.height-1 {
    height: 70px;
}

.col-2 {
    width: 144px;
}

.col-3 {
    width: 340px;
}

.col-4 {
    width: 379px;
}

.col-5 {
    width: 390px;
}

.center {
    display: flex;
    align-items: center;
    justify-content: center;
}

.pay_tools_bar {
    background-color: #fff;

    .pay_tools_bar {
        box-shadow: none;
    }
}

.cart-tit {
    padding-top: 5px;
}

.cart-head {
    border-radius: 8px;
    background: #fff;
    color: #666;
    margin-bottom: 16px;
    position: sticky;
    top: 0;
    z-index: 999;

    .cartHeadTitle {
        font-weight: 700;
        font-size: 24px;
        line-height: 24px;
        color: #1a1a1a;
        padding: 16px;
    }

    .product-list-cart {
        height: 32px;
        line-height: 32px;
        padding-bottom: 16px;
        position: relative;

        .item {
            display: flex;
            align-self: center;
            justify-content: center;
        }
    }
}

.cart {
    .block-checkbox {
        :deep(.el-checkbox__inner) {
            border: 2px solid #888b94;
            border-radius: 2.5px;
        }

        :deep(.el-checkbox__input.is-checked .el-checkbox__inner) {
            background-color: #1a1a1a;
            border-color: #1a1a1a;
        }
    }
    :deep(.el-checkbox__inner) {
        height: 18px;
        width: 18px;
    }
    :deep(.el-checkbox__inner::after) {
        width: 4px;
        height: 8px;
        border-width: 2px;
    }
}

.cart_table {
    border-radius: 8px;
    background: #fff;
    color: #666;
    margin-bottom: 8px;

    .cart_store_title {
        height: 25px;
        line-height: 25px;
        padding: 15px 0;
        display: flex;
        align-items: center;

        .item {
            color: #1a1a1a;
            .tit {
                font-size: 16px;
                font-weight: 600;
            }
        }
    }

    .cart_list_wrap {
        .product-list-cart:after {
            content: "";
            // border-bottom: 1px dashed #e6e6e6;
            position: absolute;
            left: 30px;
            right: 30px;
            bottom: 0;
        }

        .product-list-cart:last-child:after {
            display: none;
        }

        .product-list-cart {
            position: relative;

            .product-item {
                padding: 20px 0;
                color: #333;
                font-size: 14px !important;
                // &:hover {
                //     background-color: #f3f3f3;
                // }

                .item {
                    .pic {
                        position: relative;

                        img {
                            width: 70px;
                            height: auto;
                        }

                        .product-style-border {
                            position: absolute;
                            left: 0;
                            top: 0;
                            width: 70px;
                            height: 70px;
                            pointer-events: none;
                            object-fit: fill;
                        }

                        .disabled {
                            position: absolute;
                            top: 0;
                            left: 0;
                            width: 70px;
                            height: 70px;
                            background-color: rgba(0, 0, 0, 0.5);
                            color: #fff;
                            font-size: 14px;
                            display: flex;
                            align-items: center;
                            justify-content: center;
                            border-radius: 100%;
                        }
                    }

                    .name {
                        padding-left: 20px;
                        .info {
                            padding-top: 5px;
                            color: #999;
                            display: flex;
                        }
                    }

                    .money > * {
                        font-weight: bold;
                    }

                    .center-price {
                        display: flex;
                        justify-content: center;
                        .util {
                            font-size: 16px;
                            font-weight: 400;
                            line-height: 20px;
                        }
                    }

                    .icon-shanchu {
                        font-size: 24px;
                        color: #afafaf;

                        &:hover {
                            color: var(--general);
                        }
                    }
                }
            }
        }

        .disabled-item {
            background-color: transparent;

            .item {
                opacity: 0.5;
            }

            .item:last-child {
                opacity: 1;
            }

            .item.cart-invalid-item {
                opacity: 1 !important;
                color: #aaa !important;
            }

            .item.cart-invalid-product-item {
                opacity: 1;

                > .pic,
                > .name > a,
                > .name > p,
                > .name > .info,
                > .name > .extraskudata {
                    opacity: 0.5;
                }
            }

        }

        .cart-invalid-text {
            color: #aaa !important;
        }

    }
}

.table-bar-total {
    background-color: #fff;
    border-radius: 8px;

    .table-bar-total_l {
        .pay_txt_row {
            margin-left: 10px;
            color: #333;
            align-items: center;
            font-size: 16px;

            > div {
                text-align: center;
                border-left: 1px solid #dedede;
                padding: 0 20px;

                &:first-child {
                    border: none;
                }
            }

            .red {
                font-weight: bold;
            }

            .all {
                padding-right: 20px;
            }

            .font-num {
                padding: 0 4px;
            }

            .total-num {
                align-items: baseline;
            }
        }
    }
}

.tools_bar_bottom {
    bottom: 0;
    box-shadow: 0 0 10px #ccc;
    left: 0;
    min-width: 1200px;
    position: fixed;
    z-index: 12;
}

.pay_tools_bar {
    background-color: #fff;
    height: fit-content;
    border-radius: inherit;

    .pay_tools_inner {
        height: fit-content;

        .pay_tools_l {
            .pay_txt_row {
                align-items: center;
                margin-left: 10px;
                color: #333;

                > div {
                    text-align: center;
                    border-left: 1px solid #dedede;

                    &:first-child {
                        border: none;
                    }
                }

                .red {
                    font-weight: bold;
                }
            }
        }

        .pay_tools_r {
            .pay_tools_total_box {
                display: flex;
                flex-direction: column;
                max-height: 727px;
                overflow: hidden;
            }

            .settle-bottom-left {
                display: block;
            }

            .settle-final {
                margin-bottom: 16px;
                display: flex;
                justify-content: space-between;
                align-items: flex-end;

                .final-label {
                    font-size: 16px;
                    font-weight: 600;
                    color: #1f1f1f;
                    line-height: 24px;
                    margin-right: 4px;
                }

                .final-right {
                    text-align: right;
                }
            }

            .buyBtnOne {
                width: 100%;
                height: 48px;
                line-height: 1.15;
                font-size: 16px;
                font-weight: 600;
            }

            .r_prod_view {
                text-align: center;
                font-size: 14px;
                padding-right: 5px;
                margin-right: 5px;

                .red {
                    font-weight: bold;
                }
            }

            .r_price_total {
                display: flex;
                align-items: center;
                justify-content: space-between;

                :deep(.price) {
                    color: #000;
                    .util {
                        font-size: 12px;
                    }

                    .num {
                        font-size: 14px;
                        margin-left: 0px;
                    }
                }

                .tit {
                    font-size: 14px;
                    font-weight: 700;
                    margin-right: 4px;
                    line-height: 18px;
                    text-align: left;
                    color: #1a1a1a;
                }
            }

            .discounts-btn-box {
                display: flex;
                align-items: center;
                justify-content: space-between;
                font-size: 12px;
                // padding-top: 5px;
                // height: 41px;
                font-weight: 700;

                cursor: pointer;

                .discounts-btn {
                    // padding: 3px 5px;
                    // background-color: #f2f3f5;
                    border-radius: 10px;
                    // margin-left: 10px;
                    font-size: 12px;

                    .icon-shangjiantou {
                        transition: all 0.3s;
                        font-size: 12px;
                        display: inline-template;

                        &.icon-active {
                            transform: rotate(180deg);
                        }
                    }
                }
            }

            .buy-btn {
                margin-top: 10px;
                text-align: center;
                width: 100%;
                height: 44px;
                line-height: 44px;
                border-radius: 4px;
                font-size: 18px;
                font-weight: 700 !important;
                margin-right: 10px;
            }

            .checkout_btn.disabled {
                background-color: #ccc;
                cursor: default;
            }
        }
    }
}
.priceForm {
    :deep(.price) {
        .util {
            font-size: 16px;
            font-weight: bold;
        }

        .num {
            font-size: 18px;
            font-weight: bold;
            margin-left: 0px;
        }
        .decimal {
            font-size: 12px;
            margin-bottom: 2px;
        }
    }
}
.priceFormOurty {
    :deep(.price) {
        .util {
            font-size: 16px;
            font-weight: bold;
        }

        .num {
            font-size: 24px;
            font-weight: bold;
            margin-left: 0px;
        }
        .decimal {
            font-size: 16px;
            padding-top: 7px;
        }
    }
}
.flex_csb {
    display: flex;
    justify-content: space-between;
    font-size: 14px;
    font-weight: 400;
    color: #1a1a1a;
}
.no_goods {
    overflow: hidden;

    .icon {
        height: 50px;
        width: 50px;
        background-size: 69%;
    }

    .strong {
        display: flex;
        align-items: center;
        justify-content: center;
        font-size: 14px;
        font-weight: 400;
        line-height: 18px;
        text-align: center;
        color: #c2c4cc;
        flex-direction: row;
    }
    .main {
        text-align: center;

        .strong {
            color: #454545;
            font-size: 18px;
            font-weight: normal;
            line-height: 50px;
            display: flex;
            justify-content: center;
            align-items: center;
        }

        .go_buy {
            height: 28px;
            padding: 8px 12px;
            gap: 4px;
            border-radius: 4px;
            border: 0.5px solid #888b94;
            font-size: 14px;
            font-weight: 400;
            line-height: 18px;
            text-align: center;
            color: #1a1a1a;
        }
    }
}

.cart-title {
    display: flex;
    justify-content: space-between;
    gap: 16px;
    margin-top: 30px;
}

.cart-left-col {
    // 跟随内容区伸缩，填满除结算栏(358)外的空间，避免宽屏下中间留大片空白
    flex: 1;
    min-width: 0;
}

.cart_total {
    height: fit-content;
    background-color: #fff;
    width: 358px;
    top: 0;
    position: sticky;
    border-radius: 8px;
}

@media screen and (max-width: 1400px) {
    .cart-title {
        display: block;
        position: relative;
    }

    .cart-left-col,
    .r_wheit {
        width: 100%;
    }

    .r_wheit {
        min-height: 222px;
    }

    .cart_total {
        position: absolute;
        right: 0;
        top: 0;
        bottom: 0;
        width: 360px;
        pointer-events: none; /* 穿透点击事件给下方的列表 */
        background-color: transparent !important;
        border-radius: 0;
    }

    .pay_tools_bar {
        position: fixed;
        bottom: 16px;
        width: 360px;
        z-index: 9999;
        pointer-events: auto; /* 恢复点击事件 */
        box-shadow: 0 4px 20px rgba(0, 0, 0, 0.15);
        border: 1px solid #ebebeb;
        border-radius: 16px;
        overflow: hidden;
        background-color: #fff;
    }

    .pay_tools_total_box {
        max-height: none !important; /* 不限制高度，完全由 v-show 控制内容显示 */
    }

    .pay_tools_total_box.is-expanded {
        max-height: none !important;
    }

    /* 小屏幕下折叠时隐藏头部和明细面板，展开时显示 */
    .settle-header-container,
    .settle-panel-container {
        display: none !important;
    }

    .pay_tools_total_box.is-expanded {
        .settle-header-container,
        .settle-panel-container {
            display: block !important;
        }
    }

    /* 底部结算栏响应式布局：左右分布 */
    .settle-bottom-inner {
        display: flex;
        justify-content: space-between;
        align-items: center;
        width: 100%;
    }

    .settle-bottom-left {
        display: flex;
        flex-direction: column;
        align-items: flex-start;
    }

    .settle-final {
        margin-bottom: 0 !important;
        justify-content: flex-start !important;
    }

    /* 优惠明细折叠切换按钮 */
    .settle-discount-toggle {
        display: flex !important;
        align-items: flex-end;
        font-size: 12px;
        color: var(--general);
        margin-top: 4px !important;

        .discount-lbl {
            margin-right: 4px;
        }

        :deep(.price) {
            .util {
                font-size: 14px !important;
                line-height: 16px !important;
                font-weight: 400 !important;
            }
            .num {
                font-size: 14px !important;
                line-height: 18px !important;
                font-weight: 400 !important;
            }
        }

        .toggle-link {
            color: var(--general);
            display: flex !important;
            align-items: center !important;
            margin-left: 4px !important;
            cursor: pointer;

            .arrow {
                font-size: 8px;
                margin-left: 4px;

                &.up {
                    transform: rotate(180deg) !important;
                }
            }
        }
    }

    /* 结算按钮样式：圆角胶囊 */
    .buyBtnOne {
        width: 120px !important;
    }
}

.r_tit {
    font-size: 18px;
    font-weight: 700;
    color: #1a1a1a;
}
.r_wheit {
    // margin-top: 15px;
    flex: 1;
    min-width: 0;
    background: #fff;
    display: flex;
    justify-content: center;
    align-items: center;
    border-radius: 8px;
}
.name_pctyle {
    color: #1a1a1a;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
    font-size: 15px;
}
.checkRight {
    font-size: 16px;
    font-weight: 400;
    line-height: 20px;
    text-align: left;
    color: #888b94;
}
.emptied {
    cursor: pointer;
    font-size: 16px;
    font-weight: 400;
    line-height: 20px;
    text-align: left;
    color: #1a1a1a;
    padding-left: 19px;
    margin-top: 15px;
}
.buyBtnOne {
    width: 100%;
    height: 48px;
    line-height: 48px;
    font-size: 16px;
    font-weight: 600;
}

.item-label {
    font-size: 14px;
}

.allItem {
    cursor: pointer;
    font-size: 14px;
    font-weight: 400;
    line-height: 20px;
    text-align: left;
    color: #333333;
}

// 顶部批量操作：移入收藏 / 删除（对齐 Taobao 视觉）
.cart-batch-actions {
    margin-left: 16px;
    display: inline-flex;
    align-items: center;

    .batch-btn {
        cursor: pointer;
        display: inline-flex;
        align-items: center;
        justify-content: center;
        height: 32px;
        padding: 0 12px;
        border: 1px solid #ebebeb;
        border-radius: 8px;
        font-size: 14px;
        color: #1f1f1f;
        background-color: #fff;
        margin-right: 8px;
        box-sizing: border-box;

        &:hover {
            background-color: #f5f5f5;
        }
    }
}

// 结算明细面板（图4）
.settle-head {
    align-items: center;
    display: flex;
    flex-direction: row;
    justify-content: space-between;
    padding: 12px 16px 20px;

    .settle-title {
        font-size: 16px;
        line-height: 24px;
        font-weight: 700;
        color: #1a1a1a;
    }

    .settle-sub {
        font-size: 12px;
        line-height: 1.15;
        color: #999;
    }
}

.settle-header-container {
    flex-shrink: 0;
}

.settle-panel-container {
    flex: 1;
    overflow-y: auto;
    padding: 0;

    /* 美化滚动条 */
    &::-webkit-scrollbar {
        width: 6px;
    }
    &::-webkit-scrollbar-thumb {
        background: #e3e3e3;
        border-radius: 3px;
    }
    &::-webkit-scrollbar-track {
        background: transparent;
    }
}

.settle-bottom-container {
    flex-shrink: 0;
    padding: 16px;
}

.settle-discount-toggle {
    display: none; /* 大屏幕下默认隐藏，不展示折叠形态 */
}

.settle-goods-wrapper {
    padding: 0 16px;
    margin-bottom: 14px;
}

.settle-price-wrapper {
    padding: 0;

    .settle-line-item {
        padding: 0 16px;
    }
}

.settle-goods {
    align-items: flex-start;
    display: flex;
    flex-direction: row;
    flex-wrap: wrap;
    justify-content: space-between;
    overflow: hidden;
    max-height: none;

    &.is-fold {
        max-height: 168px;
    }

    .settle-goods-item {
        width: 76px;
        height: 76px;
        margin-bottom: 8px;
        border-radius: 8px;
        overflow: hidden;
        position: relative;

        &.empty {
            visibility: hidden;
        }

        .settle-goods-img-mask {
            background-color: rgba(0, 0, 0, 0.03);
            bottom: 0;
            left: 0;
            position: absolute;
            right: 0;
            top: 0;
            z-index: 1;
        }

        .settle-goods-uncheck {
            position: absolute;
            top: 0;
            right: 0;
            z-index: 9;
            background-color: rgb(0 0 0 / 50%);
            color: #fff;
            font-size: 12px;
            line-height: 20px;
            border-radius: 0 8px;
            cursor: pointer;
            text-align: center;
            width: 56px;
            transition: all 0.3s ease;
            opacity: 0;
        }

        &:hover .settle-goods-uncheck {
            opacity: 1;
        }

        :deep(.el-image) {
            width: 100%;
            height: 100%;
        }
    }
}

.settle-goods-toggle {
    align-items: center;
    cursor: pointer;
    display: flex;
    flex-direction: row;
    justify-content: center;
    margin-bottom: 2px;
    color: #7a7a7a;
    font-size: 12px;
    line-height: 24px;

    .arrow {
        display: inline-block;
        font-size: 8px;
        margin-left: 4px;

        &.up {
            transform: rotate(180deg);
        }
    }
}

.settle-lines {
    padding: 0;

    .settle-line-item {
        padding: 0 16px;
    }

    .settle-line {
        line-height: 18px;
        .lbl {
            color: #7a7a7a;
            font-size: 12px;
            line-height: 18px;
        }
    }
}

.settle-line {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 8px;
    font-size: 14px;
    line-height: 22px;

    .lbl {
        color: #666;

        &.strong {
            color: #1f1f1f;
            font-weight: 500;
        }
    }

    .val {
        color: #1f1f1f;

        &.discount-val {
            color: var(--general);
        }

        .settle-symbol {
            font-size: 12px;
            line-height: 16px;
            margin-right: 4px;
        }

        :deep(.price) {
            color: inherit;
            .util {
                font-size: 14px;
            }
            .num {
                font-size: 14px;
            }
        }
    }
}

.settle-summary-row {
    align-items: center;
    display: flex;
    justify-content: flex-end;
    margin-bottom: 8px;
    padding: 0 16px;
    font-size: 14px;
    font-weight: 500;
    line-height: 20px;
    white-space: nowrap;

    .save-total {
        color: var(--general);
        display: flex;
        align-items: baseline;
        gap: 4px;

        :deep(.price) {
            color: var(--general);
            .util {
                font-size: 12px;
                font-weight: 700;
            }
            .num {
                font-size: 14px;
                font-weight: 700;
            }
        }
    }

    .divider-line {
        background-color: rgba(0, 0, 0, 0.08);
        flex-shrink: 0;
        height: 14px;
        margin: 0 8px;
        width: 1px;
    }

    .sub-total {
        color: #1a1a1a;
        display: flex;
        align-items: baseline;
        gap: 4px;

        :deep(.price) {
            color: #1a1a1a;
            .util {
                font-size: 12px;
                font-weight: 700;
            }
            .num {
                font-size: 14px;
                font-weight: 700;
            }
        }
    }
}

.settle-final {
    margin-bottom: 16px;

    .final-label {
        font-size: 16px;
        font-weight: 600;
        color: #1f1f1f;
        line-height: 24px;
        margin-right: 4px;
    }

    .final-right {
        text-align: right;
    }
}

.discount-price {
    text-align: end;
    // o2o 共享 FormatPrice 无跨境的 .price-row 包裹层，改用其根 .price 上色
    :deep(.price) {
        color: #e1251b;
    }
}
.orgin-price {
    display: flex;
    justify-content: end;
    align-items: baseline;
    .discount-txt {
        margin-left: 4px;
        color: #888b94;
        font-family: var(--JDZhengHT-EN);
        border-radius: 2px;
        font-size: 14px;
        font-weight: 400;
        align-items: center;
        display: flex;
    }
    // o2o 共享 FormatPrice 无跨境的 .price-row 包裹层，划线价改作用于其根 .price
    :deep(.price) {
        // height: 28px;
        // line-height: 16px;
        color: #888b94;
        font-family: var(--JDZhengHT-EN);
        font-size: 16px;
        text-decoration: line-through;
        .util {
            margin-right: 0 !important;
            font-size: 16px;
        }
    }
}
:deep(.mod_you_like) {
    margin-top: 64px;
}
</style>
