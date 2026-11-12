<template>
    <div style="background-color: #f2f3f5">
        <CommonPageHeader></CommonPageHeader>
        <CommonHeader title="购物车"></CommonHeader>
        <!-- 空态：购物车无商品时展示，提供「去逛逛」入口 -->
        <div class="container" style="margin-top: 20px" v-if="!loading && cartList.length === 0">
            <div class="no_goods clearfix">
                <div class="left icon"></div>
                <div class="left main">
                    <div class="strong">{{ $t("您的购物车中暂时没有商品") }}！</div>
                    <NuxtLink to="/">
                        <el-button type="primary" size="large" :title="$t('马上挑选商品')">{{ $t("去逛逛") }}</el-button>
                    </NuxtLink>
                </div>
            </div>
        </div>
        <div class="container" v-if="!loading && cartList.length > 0" style="margin-top: 20px">
            <p class="cart-tit">{{ $t("你好，欢迎来到您的购物车") }}</p>
            <div>
                <div class="cart-head">
                    <div class="product-list-cart flex">
                        <div class="item col-1 center">
                            <el-checkbox v-model="allChecked" @change="onCheckAll"></el-checkbox>
                        </div>
                        <div class="item col-1" style="justify-content: left">{{ $t("全选") }}</div>
                        <div class="item col-3" style="justify-content: left">{{ $t("商品信息") }}</div>
                        <div class="item col-2 center">{{ $t("单价") }}</div>
                        <div class="item col-2 center">{{ $t("数量") }}</div>
                        <div class="item col-2 center">{{ $t("小计") }}</div>
                        <div class="item col-2 center">{{ $t("操作") }}</div>
                    </div>
                </div>
                <!-- 店铺分组：单个店铺（含自营）的商品行列表 + 满赠赠品区 -->
                <div class="cart_table" v-for="(item, index) in cartList" :key="index">
                    <div class="cart_store_title flex">
                        <div class="item col-1 center">
                            <el-checkbox v-model="item.isChecked" @change="onCheckAllItem(item)"></el-checkbox>
                        </div>
                        <div class="item col-3 flex align-center">
                            <div class="tag" v-if="item.shopId == 0">{{ $t("自营") }}</div>
                            <div class="tit" v-else>{{ item.shopTitle }}</div>
                        </div>
                    </div>
                    <div class="cart_list_wrap">
                        <div
                            class="product-list-cart"
                            v-for="product in item.carts"
                            :class="isDisabledData(product) ? 'disabled-item' : ''"
                            :data-wholesale-cart-id="product.wholesaleScope ? product.cartId : undefined"
                        >
                            <div class="product-item flex">
                                <div class="item col-1 center flex align-center justify-center">
                                    <el-checkbox v-model="product.isChecked" @change="onChangeCheck()" :disabled="isDisabledData(product)"> </el-checkbox>
                                </div>
                                <div class="item col-4 flex">
                                    <div class="pic">
                                        <a
                                            v-if="product.stock != 0 && product.productStatus != 0"
                                            target="_blank"
                                            :href="
                                                urlFormat({
                                                    path: 'product',
                                                    id: product.productId,
                                                    sn: product.productSn
                                                })
                                            "
                                        >
                                            <img :src="productImage(product)" :alt="product.productName || ''" />
                                            <img
                                                v-if="product.productId && styleMap[product.productId]?.borderImage"
                                                class="product-style-border"
                                                :src="styleMap[product.productId].borderImage"
                                                :alt="product.productName || ''"
                                            />
                                        </a>
                                        <div v-else>
                                            <img :src="productImage(product)" :alt="product.productName || ''" />
                                            <img
                                                v-if="product.productId && styleMap[product.productId]?.borderImage"
                                                class="product-style-border"
                                                :src="styleMap[product.productId].borderImage"
                                                :alt="product.productName || ''"
                                            />
                                        </div>
                                        <div class="disabled" v-if="shouldShowProductStatus(product, { stockField: 'stock' }) && product.isDisabled == true">
                                            <div v-if="shouldShowProductStatus(product, { stockField: 'stock' })">
                                                {{ $t(getProductStatusText(product, { stockField: "stock" })) }}
                                            </div>
                                        </div>
                                    </div>
                                    <div class="name pt5">
                                        <a
                                            v-if="product.stock != 0 && product.productStatus != 0"
                                            target="_blank"
                                            :href="
                                                urlFormat({
                                                    path: 'product',
                                                    id: product.productId,
                                                    sn: product.productSn
                                                })
                                            "
                                        >
                                            {{ product.productName }}
                                        </a>
                                        <p v-else>
                                            {{ product.productName }}
                                        </p>
                                        <div class="info">
                                            <span v-for="(sku, _index) in product.skuData"> {{ _index !== 0 ? " , " : "" }}{{ sku.name }}:{{ sku.value }}</span>
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
                                        <template v-if="product.activityInfo">
                                            <div class="promotion-box">
                                                <div
                                                    class="promotion-box-item"
                                                    v-for="subItem in formatActivityList(product.activityInfo, (item as any).usedPromotions)"
                                                    :key="subItem.promotionId"
                                                >
                                                    <div class="promotion" v-if="subItem.type != 2 && activitTextMap[subItem.type]">
                                                        <div class="ticket-content">
                                                            {{ $t(activitTextMap[subItem.type]) }}
                                                        </div>
                                                    </div>

                                                    <div class="promotion" v-if="subItem.type == 2">
                                                        <div class="ticket-type">{{ $t("券") }}<span class="ticket-line"></span></div>
                                                        <div class="ticket-content">{{ subItem.data.promotionDesc }}</div>
                                                    </div>
                                                </div>
                                            </div>
                                        </template>
                                    </div>
                                </div>
                                <div class="item col-2 center">
                                    <div class="price pt5 center-price">
                                        <FormatPrice v-model="product.price" :showText="false"></FormatPrice>
                                    </div>
                                </div>
                                <div class="item col-2 center">
                                    <div class="pt5 flex justify-center">
                                        <InputNumber
                                            v-if="isRetailLine(product)"
                                            :max="product.stock"
                                            :limitNumber="(product as any).limitNumber"
                                            v-model="product.quantity"
                                            @change="handleUpdateCartItem(product)"
                                            :disabled="isDisabledData(product)"
                                        >
                                        </InputNumber>
                                    </div>
                                </div>
                                <div class="item col-2 center">
                                    <div class="money pt5">
                                        <FormatPrice v-model="product.subtotal" :showText="false"></FormatPrice>
                                    </div>
                                </div>
                                <div class="item col-2 center">
                                    <div>
                                        <a @click="removeCartItem(product.cartId)" style="cursor: pointer">
                                            <i class="iconfont-pc icon-shanchu"></i>
                                        </a>
                                    </div>
                                </div>
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
                                                id: subItem.productId,
                                                sn: subItem.productSn
                                            })
                                        "
                                    >
                                        <div class="gift-content-item">
                                            <div class="gift-item-left">
                                                <el-image class="gift-img" lazy :src="productImage(subItem)" :alt="subItem.productName || ''"> </el-image>
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
                <!-- 底部结算栏：全选、共 N 件、删除勾选、清空、合计、优惠明细、去结算 -->
                <div class="checkOutBar" id="price-total" ref="checkoutBarComp">
                    <div :class="{ pay_tools_bar: true, tools_bar_bottom: isFixed }">
                        <div class="pay_tools_inner flex justify-between align-center container">
                            <div class="pay_tools_l flex align-center">
                                <div class="item col-1 center">
                                    <el-checkbox v-model="allChecked" @change="onCheckAll"></el-checkbox>
                                </div>
                                <div class="pay_txt_row flex">
                                    {{ $t("全选") }}
                                    <div>
                                        <span>
                                            {{ $t("共") }}
                                            <span class="font-color">
                                                {{ total.totalCount }}
                                            </span>
                                            {{ $t("件") }}
                                        </span>
                                    </div>
                                    <div>
                                        <a @click="removeCheckedCartItem">{{ $t("删除勾选商品") }}</a>
                                    </div>
                                    <div>
                                        <a @click="onClearCart">{{ $t("清空购物车") }}</a>
                                    </div>
                                </div>
                            </div>
                            <div class="pay_tools_r flex align-center">
                                <div class="pay_tools_total_box">
                                    <div class="flex align-center justify-end">
                                        <p class="r_prod_view">
                                            {{ $t("已选商品") }}
                                            <span class="font-color">{{ total.checkedCount }}</span>
                                            {{ $t("件") }}
                                        </p>

                                        <div class="r_price_total">
                                            <span class="tit">{{ $t("合计") }}：</span>
                                            <FormatPrice v-model="total.discountAfter" :showText="false"></FormatPrice>
                                        </div>
                                    </div>
                                    <div class="discounts-btn-box">
                                        <div>{{ $t("共优惠") }}：<FormatPrice v-model="total.discounts" :showText="false"> </FormatPrice></div>

                                        <CommonPopover v-model="showDiscountInfo">
                                            <template #reference>
                                                <div class="discounts-btn">
                                                    {{ $t("优惠明细") }}
                                                    <span class="iconfont-pc icon-shangjiantou" :class="{ 'icon-active': showDiscountInfo }"></span>
                                                </div>
                                            </template>
                                            <template #default>
                                                <div class="discount">
                                                    <div class="discount-title">
                                                        <div class="title-text">{{ $t("优惠明细") }}</div>
                                                    </div>
                                                    <div class="discount-content">
                                                        <div class="discount-item-box">
                                                            <div class="item-label">{{ $t("商品总额") }}</div>
                                                            <div class="item-value">
                                                                <FormatPrice :showText="false" v-model="total.productAmount" :fontStyle="{ fontWeight: 'bold' }"></FormatPrice>
                                                            </div>
                                                        </div>
                                                        <div class="discount-item-box" v-if="total.serviceFee && total.serviceFee > 0">
                                                            <div class="item-label">{{ $t("附加费用") }}</div>
                                                            <div class="item-value">
                                                                <FormatPrice v-model="total.serviceFee" :fontStyle="{ fontWeight: 'bold' }" :showText="false"></FormatPrice>
                                                            </div>
                                                        </div>
                                                        <div class="discount-item-box">
                                                            <div class="item-label">{{ $t("共优惠") }}</div>
                                                            <div class="item-value discounts-value" style="color: var(--general)">
                                                                -
                                                                <FormatPrice v-model="total.discounts" :showText="false"></FormatPrice>
                                                            </div>
                                                            <!-- <div class="sub-item-box" v-if="total.discountCouponAmount > 0">
                                                                <div class="sub-item-label">{{ $t("优惠券") }}</div>
                                                                <div class="sub-item-value discounts-value">
                                                                    -
                                                                    <FormatPrice v-model="total.discountCouponAmount"></FormatPrice>
                                                                </div>
                                                            </div> -->
                                                            <div class="sub-item-box" v-if="total.discountDiscountAmount > 0">
                                                                <!-- <div class="sub-item-label">{{ $t("其他优惠") }}</div> -->
                                                                <div class="sub-item-label">{{ $t("活动优惠") }}</div>
                                                                <div class="sub-item-value discounts-value">
                                                                    -
                                                                    <FormatPrice v-model="total.discountDiscountAmount" :showText="false"></FormatPrice>
                                                                </div>
                                                            </div>
                                                        </div>
                                                        <div class="discount-detail" v-if="total.discountCouponAmount > 0">
                                                            <div class="sub-item-label">{{ $t("优惠券") }}</div>
                                                            <div class="sub-item-value discounts-value">
                                                                -
                                                                <FormatPrice v-model="total.discountCouponAmount" :showText="false"></FormatPrice>
                                                            </div>
                                                        </div>
                                                        <div class="discount-item-box">
                                                            <div class="item-label">{{ $t("合计") }}</div>
                                                            <div class="item-value">
                                                                <FormatPrice v-model="total.discountAfter" :fontStyle="{ fontWeight: 'bold' }" :showText="false"></FormatPrice>
                                                            </div>
                                                        </div>
                                                        <div class="discount-desc">{{ $t("以上优惠不包含运费，实际优惠金额请以确认订单页为准") }}</div>
                                                    </div>
                                                </div>
                                            </template>
                                        </CommonPopover>
                                    </div>
                                </div>

                                <div @click="total.checkedCount && toCheck()" :class="'checkout_btn hand-pointer ' + (total.checkedCount == 0 ? 'disabled' : '')">
                                    {{ $t("去结算") }}
                                </div>
                            </div>
                        </div>
                    </div>
                </div>
            </div>
        </div>


        <ProdSwiper v-if="guessLikeShow" @onBuy="updateCart"></ProdSwiper>
        <div v-if="loading">
            <div>
                <div style="height: 300px; width: 100%"></div>
            </div>
        </div>
        <CommonPageFooter></CommonPageFooter>
    </div>
</template>

<script lang="ts" setup>
/**
 * 购物车页面
 * - 展示购物车商品列表，支持按店铺分组显示
 * - 提供商品选中、数量修改、删除、清空等操作
 * - 显示价格汇总、优惠明细、赠品信息
 * - 支持多种配送方式选择（普通配送/门店自提/同城配送）
 * - 结算时根据商品配送类型自动筛选可结算商品
 */
import { computed, ref, onMounted, onUnmounted } from "vue";
import { navigateTo } from "nuxt/app";
// import ProdSwiper from "./src/ProdSwiper.vue";
import ProdSwiper from "./src/ProdSwiperNew.v1.vue";
import InputNumber from "./src/InputNumber.v1.vue";
import { message, Modal } from "ant-design-vue";
import { getCart, updateCartCheck, updateCartItemData, removeCartItemData, clearCart } from "@/api/cart";
import type { CartFilterState, TotalObj, CartItmes } from "@/types/cart/cart.d";
import { ElLoading } from "element-plus";
import { useCommonStore } from "@/store/common";
import { urlFormat } from "@/utils/format";
import { getProductStatusText, shouldShowProductStatus } from "@/utils/productSoldOut";
import { useProductStyle } from "~/hooks";

const commonStore = useCommonStore();

const { t } = useI18n();


/** 是否按零售计步器渲染该行；关闭批发时恒 true（全部走零售控件） */
const isRetailLine = (product: CartItmes): boolean => {
    return true;
};

const loading = ref(true);
const cartList = ref<CartFilterState[]>([]);
const cartProducts = computed(() => cartList.value.flatMap((item) => item.carts ?? []));
const { styleMap } = useProductStyle(cartProducts);
const total = ref<TotalObj>({} as TotalObj);
const guessLikeShow = ref(false);
const cartLoading = ref();
const showDiscountInfo = ref(false);

onMounted(() => {
    updateCart();
});

/**
 * @description 获取购物车列表数据，包括商品信息、优惠活动和统计金额
 */
const updateCart = async () => {
    try {
        cartLoading.value = ElLoading.service({
            lock: false,
            text: t("请求加载中"),
            customClass: "tig-normal-loading"
        });
        const result = await getCart();
        cartList.value = result.cartList;
        total.value = result.total;
        // console.log("total", total.value);
        updateCheckbox();
    } catch (error) {
    } finally {
        loading.value = false;
        cartLoading.value.close();
        guessLikeShow.value = true;
    }
};

/**
 * @description 将选中状态同步到服务端，确保刷新页面后选中状态一致
 */
const updateCheckData = async () => {
    try {
        const checkData = <{ cartId: number; isChecked: number }[]>[];
        cartList.value.forEach((item) => {
            item.carts.forEach((product) => {
                checkData.push({
                    cartId: product.cartId,
                    isChecked: product.isChecked && product.isDisabled === false ? 1 : 0
                });
            });
        });
        const result = await updateCartCheck({ data: checkData });
        updateCart();
    } catch (error) {
    } finally {
    }
};

/**
 * @description 处理商品数量变更，带防抖逻辑
 * @param product 商品项
 */
const handleUpdateCartItem = (product: CartItmes) => {
    startDelayTimer(product);
};
let delayTimer: any; // 延时定时器

/**
 * @description 延时触发购物车商品数量更新，防止频繁请求
 * @param product 商品项
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
 * @description 实际执行购物车商品数量更新请求
 * @param product 商品项
 */
const updateCartItem = async (product: CartItmes) => {
    try {
        await updateCartItemData(product.cartId, {
            quantity: product.quantity
        });
        updateCart();
    } catch (error: any) {
        console.log(error);
        message.error(error.message);
        updateCart();
    }
};

/**
 * @description 删除单个购物车商品，弹窗确认后执行删除
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
 * @description 删除所有选中的商品，弹窗确认后批量删除
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
        message.error(t("您还未选中任何商品"));
    }
};

/**
 * @description 清空购物车所有商品，弹窗确认后执行清空
 */
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

/**
 * @description 单个商品选中状态变更处理
 */
const onChangeCheck = () => {
    updateCheckData();
};

/**
 * @description 全选/取消全选购物车商品，仅操作未禁用的商品
 */
const onCheckAll = () => {
    cartList.value.forEach((item) => {
        item.carts.forEach((product) => {
            product.isChecked = allChecked.value == true && product.isDisabled === false ? true : false;
        });
    });
    updateCheckbox();
    updateCheckData();
};

/**
 * @description 店铺级全选/取消全选，批量选中或取消选中某店铺下所有商品
 * @param item 店铺购物车数据
 */
const onCheckAllItem = (item: CartFilterState) => {
    item.carts.forEach((product) => {
        product.isChecked = item.isChecked == true ? true : false;
    });
    updateCheckbox();
    updateCheckData();
};

const allChecked = ref(false);

/**
 * @description 根据商品选中状态更新店铺级和全局全选状态
 */
const updateCheckbox = () => {
    cartList.value.forEach((item) => {
        const availableProducts = item.carts.filter((product) => !product.isDisabled);
        item.isChecked = availableProducts.every((product) => product.isChecked === true && !product.isDisabled);
    });
    allChecked.value = cartList.value.every((item) => item.isChecked === true);
};

/**
 * @description 判断商品是否禁用（已下架、已售罄或无效商品）
 */
const isDisabledData = (product: CartItmes) => {
    if (product.productStatus == 0 || product.stock == 0 || product.isDisabled == true) {
        return true;
    }
    return false;
};

interface ActivitTextMap {
    [key: number]: string;
}
const activitTextMap: ActivitTextMap = {
    1: "秒杀价",
    2: "券",
    3: "满减",
    4: "折扣",
    5: "满赠",
    6: "限时折扣",
    7: "拼团"
};

/**
 * @description 格式化商品活动标签列表，优先展示活动 + 已用优惠券
 */
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

const checkoutBarComp = ref<any>(null);
const isFixed = ref(false);
const updateAttrActionFixed = () => {
    const scrollY = window.scrollY;
    const windowHeight = window.innerHeight;
    const checkOutBar = checkoutBarComp.value;

    if (!checkOutBar) return;
    const checkOutBarTop = checkOutBar.getBoundingClientRect().top + scrollY;
    isFixed.value = !(scrollY + windowHeight - 60 > checkOutBarTop);
};

onMounted(() => {
    window.addEventListener("scroll", updateAttrActionFixed);
    updateAttrActionFixed();
});

onUnmounted(() => {
    window.removeEventListener("scroll", updateAttrActionFixed);
});

/**
 * @description 去结算，检查是否有O2O配送选项需要选择（自提/同城配送）
 */
const toCheck = () => {
    if (commonStore.closeOrder === 1) return message.error(t("商城已关闭下单"));
};

</script>

<style lang="less" scoped>
.cart-tit {
    padding-top: 5px;
}

.col-1 {
    width: 60px;
}

.col-2 {
    width: 180px;
}

.col-3 {
    width: 340px;
}

.col-4 {
    width: 400px;
}

.center {
    display: flex;
    align-items: center;
    justify-content: center;
}

.cart-head {
    border: 1px solid #dfdfdf;
    border-radius: 5px;
    background: #fff;
    color: #666;
    margin: 15px 0;

    .product-list-cart {
        height: 40px;
        line-height: 40px;
        position: relative;

        .item {
            display: flex;
            align-self: center;
            justify-content: center;
        }
    }
}

/* 空态 */
.no_goods {
    margin-top: 20px;
    overflow: hidden;
    height: 380px;
    width: 100%;
    display: flex;
    justify-content: center;
    align-items: center;

    .icon {
        background: url("/assets/images/flow/flow_cart.png") no-repeat scroll 0 -43px transparent;
        height: 93px;
        width: 161px;
    }

    .main {
        .strong {
            color: #454545;
            font-size: 18px;
            font-weight: normal;
            line-height: 50px;
        }

        .go_buy {
            padding: 8px 20px;
            font-size: 14px;
        }
    }
}

/* 店铺分组 */
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
        display: grid;
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

    .promotion-box-item {
        margin-right: 10px;
    }

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
            max-width: 90px;
            overflow: hidden;
            text-overflow: ellipsis;
            white-space: nowrap;
        }
    }
}

.cart_table {
    border: 1px solid #dfdfdf;
    border-radius: 5px;
    background: #fff;
    color: #666;
    margin-bottom: 20px;

    .cart_store_title {
        border-bottom: 1px solid #dcdcdc;
        height: 25px;
        line-height: 25px;
        padding: 15px 0;
        display: flex;
        align-items: center;

        .item {
            color: #333;

            .tag {
                background-color: var(--main-bg);
                border-radius: 2px;
                color: #fff;
                font-size: 12px;
                height: 16px;
                line-height: 16px;
                margin-right: 10px;
                padding: 0 4px;
            }
        }
    }

    .cart_list_wrap {
        .product-list-cart:after {
            content: "";
            border-bottom: 1px dashed #e6e6e6;
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

                &:hover {
                    background-color: #f3f3f3;
                }

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
            background-color: #f2f2f2;

            .item {
                opacity: 0.5;
            }

            .item:last-child {
                opacity: 1;
            }
        }
    }
}

/* 底部结算栏 */
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
        padding: 10px 0;

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
            color: #969799;
            font-size: 12px;
        }

        &:last-child {
            padding-top: 0;
        }
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

    .discount-desc {
        font-size: 12px;
        color: #969799;
        letter-spacing: 0;
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
    box-shadow: 0 0 10px #ccc;
    background-color: #fcfcfc;
    width: 100%;
    height: 60px;

    .pay_tools_bar {
        box-shadow: none;
    }

    .pay_tools_inner {
        height: 60px;

        .pay_tools_l {
            .pay_txt_row {
                margin-left: 10px;
                color: #333;

                > div {
                    width: 100px;
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
                padding-right: 20px;
            }

            .r_prod_view {
                text-align: center;
                font-size: 14px;
                padding-right: 5px;
                margin-right: 5px;
                border-right: 1px solid #dedede;

                .red {
                    font-weight: bold;
                }
            }

            .r_price_total {
                display: flex;
                align-items: center;

                :deep(.price) {
                    color: var(--price);

                    .util {
                        font-size: 12px;
                        font-weight: bold;
                    }

                    .num {
                        font-family: "Century Gothic", "微软雅黑", "Tahoma";
                        font-size: 16px;
                        font-weight: bold;
                        margin-left: 0px;
                    }
                }

                .tit {
                    font-size: 14px;
                }
            }

            .discounts-btn-box {
                display: flex;
                align-items: center;
                justify-content: end;
                font-size: 12px;
                padding-top: 5px;
                cursor: pointer;

                .discounts-btn {
                    padding: 3px 5px;
                    background-color: #f2f3f5;
                    border-radius: 10px;
                    margin-left: 10px;
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

            .checkout_btn {
                background-color: var(--main-bg);
                color: var(--main-text);
                font-family: "微软雅黑";
                font-size: 17px;
                font-weight: bold;
                text-align: center;
                width: 110px;
                height: 60px;
                line-height: 60px;
            }

            .checkout_btn.disabled {
                background-color: #ccc;
                cursor: default;
            }
        }
    }
}
</style>
