<template>
    <div
        class="product_info"
        :class="{
            'o2o-wholesale-compact': isWholesale && isStore(),
            'wholesale-no-store-spacing': isWholesale && !isStore()
        }"
    >
        <div :class="['basic', !activityPcImage ? 'bg' : '']" v-if="productType == 'exchange'">
            <div class="item exchange" :class="{ 'has-activity-bg active-box': !!activityPcImage }" :style="{ alignItems: 'center', ...activeBoxStyle }">
                <div class="tit">{{ $t("积分兑换") }}</div>
                <div class="price1">
                    <FormatPrice v-model="exchangeDetail.discountsPrice"></FormatPrice>
                    <span class="red">+</span>
                    &nbsp;&nbsp;
                    <span class="exchange-num red">{{ exchangeDetail.exchangeIntegral }}</span>
                    <span class="red">{{ $t("积分") }}</span>
                </div>
            </div>
        </div>
        <template v-else>
            <template v-if="isSeckill == 1 && !isWholesale">
                <div class="active-box flex" :class="{ 'has-activity-bg': !!activityPcImage }" :style="{ alignItems: 'center', ...activeBoxStyle }">
                    <div class="active-ico flex align-center">
                        <!-- <i class="iconfont-pc icon-miaosha"></i> -->
                        <span>{{ $t("秒杀") }}</span>
                    </div>
                    <div class="active-time">
                        <Countdown :endTime="seckillValue.seckillEndTime"></Countdown>
                    </div>
                </div>
            </template>
            <template v-if="!isPresale && isDiscount == 1 && !isWholesale">
                <div class="active-box flex" :class="{ 'has-activity-bg': !!activityPcImage }" :style="{ alignItems: 'center', ...activeBoxStyle }">
                    <div class="active-ico flex align-center">
                        <!-- <i class="iconfont-pc icon-miaosha"></i> -->
                        <span>{{ $t("限时折扣") }}</span>
                    </div>
                    <div class="active-time">
                        <Countdown :endTime="discountValue.discountEndTime"></Countdown>
                    </div>
                </div>
            </template>
            <template v-if="isPresale">
                <div class="active-box flex" :class="{ 'has-activity-bg': !!activityPcImage }" :style="{ alignItems: 'center', ...activeBoxStyle }">
                    <div class="active-ico flex align-center">
                        <!-- <i class="iconfont-pc icon-miaosha"></i> -->
                        <span>{{ $t("预售") }}</span>
                    </div>
                </div>
            </template>
            <template v-if="isNormalWithActivityBg">
                <div class="active-box active-box-normal has-activity-bg flex align-center" :style="{ ...activeBoxStyle }">
                    <div class="active-price-info flex align-center">
                        <div class="price-group" v-if="productPrice !== ''">
                            <FormatPrice
                                v-model="productPrice"
                                :currencyStyle="{ fontSize: '24px', fontWeight: '700' }"
                                :font-style="{ fontSize: '36px', fontWeight: '700' }"
                            ></FormatPrice>
                        </div>
                        <div class="price-group market" v-if="product.marketPrice && Number(product.marketPrice) > 0 && (commonStore.showMarketprice || isSupplierProduct)">
                            <FormatPrice v-model="product.marketPrice" :font-style="{ 'text-decoration': 'line-through' }"></FormatPrice>
                        </div>
                    </div>
                </div>
            </template>
        </template>
        <div class="product_name">
            <div class="name" :class="{ 'name--with-desc': Boolean(product.productBrief) }">
                <span>{{ product.productName }}</span>
            </div>
            <p v-if="product.productBrief" class="desc">
                {{ product.productBrief }}
            </p>
        </div>
        <!-- 预售价格信息 -->
        <div class="basic bg presale-basic" v-if="productType != 'exchange' && isPresale">
            <div class="item main-price-box presale-price-box">
                <!-- 定金预售：展示「定金」+「预售价」两行 -->
                <template v-if="presaleDetailState.saleMode === 2">
                    <div class="presale-line">
                        <div class="tit">{{ $t("定金") }}:</div>
                        <div class="price1">
                            <FormatPrice
                                :model-value="presaleDetailState.depositAmount"
                                :currencyStyle="{ fontSize: '14px', color: '#B4B4B4' }"
                                :font-style="{ fontSize: '18px', color: '#B4B4B4' }"
                            ></FormatPrice>
                        </div>
                    </div>
                    <div class="presale-line">
                        <div class="tit">{{ $t("预售价") }}:</div>
                        <div class="price1 theme">
                            <FormatPrice
                                :model-value="presaleTotalPrice"
                                :currencyStyle="{ fontSize: '16px', fontWeight: '700' }"
                                :font-style="{ fontSize: '28px', fontWeight: '700' }"
                            ></FormatPrice>
                        </div>
                    </div>
                </template>
                <!-- 全款预售：仅展示预售价 -->
                <template v-else>
                    <div class="presale-line">
                        <div class="tit">{{ $t("预售价") }}:</div>
                        <div class="price1 theme">
                            <FormatPrice
                                :model-value="presalePriceValue"
                                :currencyStyle="{ fontSize: '16px', fontWeight: '700' }"
                                :font-style="{ fontSize: '28px', fontWeight: '700' }"
                            ></FormatPrice>
                        </div>
                    </div>
                </template>
            </div>
        </div>

        <!-- 评价 + 销量（普通商品 + 预售商品统一展示） -->
        <div class="basic bg" v-if="productType != 'exchange'">
            <!-- D1146：评价功能关闭时不展示评分行 -->
            <div class="flex pg_16" v-if="commonStore.productCommentEnabled === 1 && rankDetail.total > 0">
                <div class="star">
                    <span class="icon-star" :class="`icon-star-${rankDetail.averageRank}`"></span>
                </div>
                <div class="evaluate">
                    {{ $t("已有") }} <span class="font-color"> {{ rankDetail.total }}</span> {{ $t("人评价") }}，{{ rankDetail.goodPercent }}% {{ $t("好评") }}
                </div>
            </div>
            <div class="item" v-if="commonStore.showSelledCount">
                <div class="num">{{ $t("已售") }}{{ isSeckill == 1 ? seckillValue.seckillSales : product.virtualSales }} {{ $t("件") }}</div>
            </div>
            <!-- 尾款信息（定金预售）- 移到已售多少件下方 -->
            <div class="item presale-final" v-if="isPresale && presaleDetailState.saleMode === 2">
                <div class="presale-final-grid">
                    <div class="tit">{{ $t("尾款") }}:</div>
                    <div class="price1">
                        <FormatPrice
                            :model-value="Number(presaleDetailState.finalPaymentAmount || 0)"
                            :currencyStyle="{ fontSize: '12px' }"
                            :font-style="{ fontSize: '14px' }"
                        ></FormatPrice>
                    </div>
                    <!-- <div v-if="finalPaymentTimeDisplay" class="presale-time">({{ finalPaymentTimeDisplay }})</div> -->
                    <div v-if="presaleDetailState.finalPaymentStartTimeText" class="presale-time">({{ presaleDetailState.finalPaymentStartTimeText }})</div>
                </div>
            </div>
            <!-- 流程/发货 - 移到已售多少件下方 -->
            <div class="item presale-process" v-if="isPresale && presaleDetailState.saleMode === 2">
                <div class="tit">{{ $t("流程") }}:</div>
                <div class="process-text">1.{{ $t("付定金") }}-2.{{ $t("付尾款") }}-3.{{ $t("发货") }}</div>
                <el-popover
                    placement="bottom"
                    :width="500"
                    trigger="hover"
                    popper-style="box-shadow: rgb(14 18 22 / 35%) 0px 10px 38px -10px, rgb(14 18 22 / 20%) 0px 10px 20px -15px; padding: 20px;"
                    :teleported="false"
                >
                    <template #reference>
                        <i class="iconfont-pc icon-tishi presale-info-icon"></i>
                    </template>
                    <template #default>
                        <div class="presale-rule-content">
                            <div class="rule-section">
                                <div class="rule-title">{{ $t("下单规则") }}</div>
                                <div class="rule-item">
                                    <span class="rule-number">1、</span>
                                    <span>{{
                                        $t(
                                            "预售商品，有营销（会员价、优惠券等）时自动推荐使用，若为定金预售则在营销（会员价、优惠券等）的基础上计算尾款，付尾款时以定金的价格为基础计算尾款金额；"
                                        )
                                    }}</span>
                                </div>
                                <div class="rule-item">
                                    <span class="rule-number">2、</span>
                                    <span>{{ $t("订单优惠信息将在尾款时计算") }}</span>
                                </div>
                            </div>
                            <div class="rule-section">
                                <div class="rule-title">{{ $t("售后规则") }}</div>
                                <div class="rule-item">
                                    <span class="rule-number">1、</span>
                                    <span>{{ $t("全款预售商品，支付预售款后可以正常发起售后流程；") }}</span>
                                </div>
                                <div class="rule-item">
                                    <span class="rule-number">2、</span>
                                    <span>{{ $t("定金预售商品，未付尾款的订单到期也可支付尾款；付尾款后，可正常申请售后流程") }}</span>
                                </div>
                            </div>
                        </div>
                    </template>
                </el-popover>
            </div>
            <div class="item presale-process" v-else-if="isPresale">
                <div class="tit">{{ $t("发货") }}</div>
                <div class="process-text">
                    <!-- {{ formatDeliveryTime }} -->
                    {{ presaleDetailState.deliveryTimeText }}
                </div>
            </div>
        </div>

        <!-- 普通商品价格信息 -->
        <div class="basic bg" v-if="productType != 'exchange' && !isPresale">
            <div class="item main-price-box" v-if="!isNormalWithActivityBg">
                <div class="price1" :class="{ theme: isSeckill == 1 }" v-if="!isWholesale && productPrice !== ''">
                    <FormatPrice
                        v-model="productPrice"
                        :currencyStyle="{ fontSize: '24px', fontWeight: '700' }"
                        :font-style="{ fontSize: '36px', fontWeight: '700' }"
                    ></FormatPrice>
                </div>
                <div class="flex" v-if="!isWholesale && isSeckill != 1 && productPrice !== '' && (commonStore.showMarketprice || isSupplierProduct)">
                    <template v-if="product.marketPrice && Number(product.marketPrice) > 0">
                        <div class="market_price">
                            <FormatPrice v-model="product.marketPrice" :font-style="{ 'text-decoration': 'line-through' }"></FormatPrice>
                        </div>
                    </template>
                </div>
            </div>
            <div class="item" v-if="!isWholesale">
                <ProductCouponList :promotionList="promotionList" :productId="product.productId" :shopId="product.shopId"> </ProductCouponList>
            </div>
        </div>

        <div class="basic">
            <!--            <div class="item">-->
            <!--                <div class="tit">{{$t('送货至')}}：</div>-->
            <!--                <div class="address">-->
            <!--                    <SelectRegion v-model="regionIds" @change="onRegionIdsChange"></SelectRegion>-->
            <!--                </div>-->
            <!--            </div>-->
            <!-- <template v-if="serviceList.length > 0">
                <div class="item align-start">
                    <div class="tit">{{ $t("服务") }}：</div>
                    <div class="tips_i_box">
                        <template v-for="(item, index) in serviceList" :key="index">
                            <template v-if="item.icoImg">
                                <span> <img :src="imageFormat(item.icoImg)" width="15" height="15" />&nbsp;{{ item.productServiceName }} </span>
                            </template>
                        </template>
                    </div>
                </div>
            </template> -->
            <template v-if="product.productType === 7 || product.productType === 8">
                <div class="item">
                    <div class="tit">{{ $t("说明") }}：</div>
                    <div class="num" style="font-size: 12px">{{ $t("此商品不支持售后，如需售后请联系客服") }}</div>
                </div>
            </template>
        </div>


        <template v-if="isB2B() && skuList.length > 1 && commonStore.bulkPurchase === 1 && !isWholesale">
            <ProductB2bTab v-model:tabIndex="tabIndex"></ProductB2bTab>
        </template>

        <div class="actions" :class="{ 'no-border': isB2B() && skuList.length > 1 && commonStore.bulkPurchase === 1 && !isWholesale }">
            <template v-if="!isWholesale">
            <template v-if="tabIndex !== 1">
                <template v-if="skuList.length > 0">
                    <ProductSku v-model="attrList" :skuList="skuList" :checkedValue="checkedValue" :productType="productType" @change="onProductSkuChange">
                    </ProductSku>
                </template>
                <template v-if="!(product.productType === 4 && product.isBuy === 1)">
                    <div class="number-box">
                        <div class="title">{{ $t("数量") }}</div>
                        <div class="con">
                            <InputNumber
                                class="conInput"
                                v-model="productNumber"
                                :max="maxProductNumber > 0 ? maxProductNumber : productStock > 0 ? productStock : 1"
                            ></InputNumber>
                            <div class="flex" style="margin-left: 10px">
                                {{ $t("件") }}
                                <div style="color: #999">
                                    （<ProductStockShow v-model="productStock" :isSeckill="isSeckill" :maxProductNumber="maxProductNumber"> </ProductStockShow
                                    >）
                                </div>
                            </div>
                        </div>
                    </div>
                    <!-- 二期会员标签：不在人群范围，数量选择下方红字提示 -->
                    <div class="member-restrict-tip" v-if="productType == 'product' && memberRestricted">
                        <i class="iconfont-pc icon-tishi"></i>
                        <span>{{ memberRestrictTip }}</span>
                    </div>
                </template>
            </template>
            <template v-if="isBatch">
                <ProductB2bSku
                    v-model:sku-item="skuItem"
                    v-model:select-skus="selectSkus"
                    v-model:product-amount="productAmount"
                    :id="product.productId"
                    :b2bSkuList="b2bSkuList"
                ></ProductB2bSku>
            </template>
            <ProductAttr v-model="attrList" @change="handleAttrChange"> </ProductAttr>
            <template v-if="isBatch && productAmount && productAmount.count > 0">
                <ProductAmount :select-skus="selectSkus" :product-amount="productAmount"></ProductAmount>
            </template>
            <Teleport defer :to="buyActionTarget || 'body'" :disabled="!buyActionTarget">
                <div class="actions external-buy-actions">
                    <template v-if="product.productType === 4 && product.isBuy === 1">
                        <div class="add_cart">
                            <TigButton style="width: 50%" theme :disabled="true" class="buy-btn buynow" type="primary">{{ $t("商品已购买") }}</TigButton>
                        </div>
                    </template>
                    <template v-else>
                        <div class="add_cart">
                    <template v-if="isBatch">
                        <ProductBuy :extraAttrIds="attrIds" :id="product.productId" :skuItem="skuItem" :disabled="skuItem.length === 0" :isQuick="true">
                            <TigButton style="white-space: nowrap" theme :disabled="skuItem.length === 0" class="buy-btn buynow" type="primary">{{
                                isPresale ? (presaleDetailState.saleMode === 2 ? $t("立即预定") : $t("立即预购")) : $t("立即购买")
                            }}</TigButton>
                        </ProductBuy>
                        <ProductBuy v-if="!isPresale" :extraAttrIds="attrIds" :id="product.productId" :disabled="skuItem.length === 0" :skuItem="skuItem">
                            <TigButton style="white-space: nowrap" :disabled="skuItem.length === 0" class="buy-btn addToCart">{{
                                $t("批量加入购物车")
                            }}</TigButton>
                        </ProductBuy>
                    </template>
                    <template v-else>
                        <!-- 二期会员标签：不在人群范围，购买入口置灰，文案与可购买态保持一致 -->
                        <template v-if="productType == 'product' && memberRestricted">
                            <TigButton theme disabled class="buy-btn buynow" type="primary" style="white-space: nowrap">
                                {{ isPresale ? (presaleDetailState.saleMode === 2 ? $t("立即预定") : $t("立即预购")) : $t("立即购买") }}
                            </TigButton>
                            <template v-if="!isPresale && ((isSeckill == 1 && productStock == 0) || product.productType === 1)">
                                <TigButton disabled class="buy-btn" v-if="isSeckill == 1 && productStock == 0">{{ $t("已抢完") }}</TigButton>
                                <el-button disabled class="buy-btn addToCart" v-else-if="product.productType === 1">{{ $t("加入购物车") }}</el-button>
                            </template>
                        </template>
                        <template v-else-if="productType == 'product'">
                            <template v-if="productStock == 0">
                                <TigButton theme class="buy-btn buynow" type="primary" @click="$router.push('/search')">{{ $t("查看其他商品") }}</TigButton>
                            </template>
                            <template v-else>
                                <ProductBuy
                                    :extraAttrIds="attrIds"
                                    :id="product.productId"
                                    :skuId="skuId"
                                    :disabled="productStock == 0"
                                    :number="productNumber"
                                    :isQuick="true"
                                >
                                    <TigButton
                                        v-if="isPresale"
                                        theme
                                        :disabled="productStock == 0"
                                        class="buy-btn buynow"
                                        type="primary"
                                        style="white-space: nowrap"
                                    >
                                        <template v-if="presaleDetailState.saleMode === 2">
                                            <FormatPrice
                                                :model-value="Number(presalePriceValue || 0)"
                                                :fontStyle="{ fontSize: '18px', fontWeight: '700', color: '#fff' }"
                                                :currencyStyle="{ fontSize: '14px', fontWeight: '700', color: '#fff' }"
                                                :decimalsStyle="{ fontSize: '18px', fontWeight: '700', color: '#fff' }"
                                            ></FormatPrice>
                                            <span>{{ $t("立即预定") }}</span>
                                        </template>
                                        <template v-else>
                                            {{ $t("立即预购") }}
                                        </template>
                                    </TigButton>
                                    <TigButton v-else theme :disabled="productStock == 0" class="buy-btn buynow" type="primary">{{ $t("立即购买") }}</TigButton>
                                </ProductBuy>
                            </template>

                            <template v-if="!isPresale && ((isSeckill == 1 && productStock == 0) || product.productType === 1)">
                                <ProductBuy
                                    :extraAttrIds="attrIds"
                                    :id="product.productId"
                                    :skuId="skuId"
                                    :disabled="productStock == 0"
                                    :number="productNumber"
                                >
                                    <TigButton :disabled="productStock == 0" class="buy-btn" v-if="isSeckill == 1 && productStock == 0">{{
                                        $t("已抢完")
                                    }}</TigButton>
                                    <el-button :disabled="productStock == 0" class="buy-btn addToCart" v-else-if="product.productType === 1">{{
                                        $t("加入购物车")
                                    }}</el-button>
                                </ProductBuy>
                            </template>
                        </template>
                        <template v-if="productType == 'exchange'">
                            <ProductBuy
                                :extraAttrIds="attrIds"
                                :id="exchangeDetail.id"
                                :skuId="skuId"
                                :disabled="productStock == 0"
                                :number="productNumber"
                                :productType="productType"
                                :isQuick="true"
                            >
                                <TigButton theme class="buy-btn buynow" type="primary" :disabled="productStock == 0">{{ $t("立即兑换") }}</TigButton>
                            </ProductBuy>
                        </template>
                    </template>
                    <!-- 立即询价：会员限制时同步置灰，缺货时仍可询价 -->
                    <template v-if="!isPresale && commonStore.isEnquiry === 1 && (isB2B() || isOverseas() || isS2b2c())">
                        <TigButton
                            style="width: 50%; padding: 0"
                            class="buy-btn buynow"
                            :disabled="productType == 'product' && memberRestricted"
                            :title="productType == 'product' && memberRestricted ? memberRestrictTip : ''"
                            @click="showEnquiry = !showEnquiry"
                            >{{ $t("立即询价") }}</TigButton
                        >
                    </template>
                        </div>
                    </template>
                </div>
            </Teleport>
            <template v-if="serviceList && serviceList.length > 0">
                <div class="serviceAction">
                    <el-collapse v-model="activeName">
                        <template v-for="(item, index) in serviceList" :key="index">
                            <el-collapse-item :title="item.productServiceName">
                                <template #icon="{ isActive }">
                                    <span class="collapse-icon" :class="{ active: isActive }">
                                        <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="none" class="arrow-icon">
                                            <path
                                                stroke="#888B94"
                                                stroke-linecap="round"
                                                stroke-linejoin="round"
                                                stroke-width="1.5"
                                                d="M11.5 6.25 8 9.75l-3.5-3.5"
                                            ></path>
                                        </svg>
                                    </span>
                                </template>
                                <div class="basic">
                                    <template v-if="serviceList.length > 0">
                                        <div class="item align-start">
                                            <div class="tips_i_box">
                                                <span>{{ item.productServiceDesc || "" }} </span>
                                            </div>
                                        </div>
                                    </template>
                                </div>
                            </el-collapse-item>
                        </template>
                    </el-collapse>
                </div>
            </template>
            </template>
        </div>
        <ProductEnquiry v-model="showEnquiry" :productId="product.productId"></ProductEnquiry>
    </div>
</template>
<script lang="ts" setup>
import { reactive, ref, onMounted } from "vue";
import InputNumber from "~/components/product/src/InputNumber.vue";
import Countdown from "./Countdown.vue";
import { SelectRegion } from "@/components/region";
import { getProductSkuDetail } from "~/api/product/product";
import { imageFormat } from "@/utils/format";
import type { ProductItem, AttrList, SkuList, ServiceList, SkuPromotion, ExchangeDetail, ProductAmountItem } from "~/types/product/product.d";
import { useCommonStore } from "~/store/common";
// 显式引入跨境版子组件副本，覆盖 o2o 全局自动导入的同名组件，确保渲染与 nextrade-overseas-pc 一致
import ProductSku from "~/components/product/ProductSku.vue";
import ProductAttr from "~/components/product/ProductAttr.vue";
import ProductBuy from "~/components/product/Buy.vue";
import ProductEnquiry from "~/components/product/ProductEnquiry.vue";
import ProductB2bTab from "~/components/product/B2bTab.vue";
import ProductB2bSku from "~/components/product/B2bSku.vue";
import ProductAmount from "~/components/product/ProductAmount.vue";
import ProductStockShow from "~/components/product/StockShow.vue";
import ProductCouponList from "./CouponList.vue";

const props = defineProps({
    product: {
        type: Object as PropType<ProductItem>,
        default: {}
    },
    attrList: {
        type: Object as PropType<AttrList>,
        default: {}
    },
    exchangeDetail: {
        type: Object as PropType<ExchangeDetail>,
        default: {}
    },
    skuList: {
        type: Array as PropType<SkuList[]>,
        default: []
    },
    b2bSkuList: {
        type: Array as PropType<SkuList[]>,
        default: []
    },
    checkedValue: {
        type: Array as PropType<string[]>,
        default: []
    },
    serviceList: {
        type: Array as PropType<ServiceList[]>,
        default: []
    },
    rankDetail: {
        type: Object,
        default: {}
    },
    productPrice: {
        type: String,
        default: ""
    },
    productNumber: {
        type: Number,
        default: 1
    },
    skuId: {
        type: Number,
        default: 0
    },
    id: {
        type: Number,
        default: 0
    },
    productType: {
        type: String,
        default: "product"
    },
    presaleDetail: {
        type: Object,
        default: () => ({})
    },
    // 批发态总开关（由 Product.vue 传入）：true 时价格/规格/购买区切换为批发形态并去营销化。
    // 注意：该 prop 始终存在（非批发态为 false），以便零售模板用 v-if="!isWholesale" 兜底。
    isWholesale: {
        type: Boolean,
        default: false
    },
    // 详情接口下发的批发阶梯价表；常驻 prop，非批发态为 null
    wholesalePriceList: {
        type: Array as PropType<any[] | null>,
        default: null
    },
    activityPcImage: {
        type: String as PropType<string | null>,
        default: null
    },
    // O2O 当前履约门店信息（用于门店切换卡片展示）
    storeInfo: {
        type: Object,
        default: () => ({})
    },
    // 购买按钮挂载到右栏滚动区外；未传时保持组件原有的内联布局。
    buyActionTarget: {
        type: String,
        default: ""
    }
});

const productStock = defineModel("productStock", {
    type: Number,
    default: 1
});
const showEnquiry = defineModel("showEnquiry", {
    type: Boolean,
    default: false
});

const attrList = ref<object>(props.attrList);
const exchangeDetail = ref<ExchangeDetail>(props.exchangeDetail);
const productPrice = ref<any>(props.productPrice);
const isSeckill = ref<number>(0);
const maxProductNumber = ref(0);
const isDiscount = ref(0);
const productNumber = ref<number>(props.productNumber);
const skuId = ref<number>(props.skuId);
const regionIds = ref<any[]>([]);
const promotionList = ref<SkuPromotion[]>([]);
const seckillValue = ref({
    seckillEndTime: "",
    seckillSales: 0,
    seckillStock: 0,
    canJoin: true,
    restrictTip: ""
});
const discountValue = ref({
    discountEndTime: "",
    discountPrice: 0,
    canJoin: true,
    restrictTip: ""
});
const productAmount = ref<ProductAmountItem>({
    count: 0,
    total: "0"
});
const selectSkus = ref<any[]>([]);
const skuItem = ref<any[]>([]);
// const showEnquiry = ref(true);
const commonStore = useCommonStore();
const activeName = ref("");
const { t } = useI18n();
const route = useRoute();
const router = useRouter();



const presaleDetailState = ref<any>({});
watch(
    () => props.presaleDetail,
    (val) => {
        presaleDetailState.value = val ? { ...(val as any) } : {};
    },
    { deep: true, immediate: true }
);

const activeBoxStyle = computed(() => {
    if (props.activityPcImage) {
        return {
            background: `url(${imageFormat(props.activityPcImage)}) no-repeat center center`,
            backgroundSize: "cover",
            width: "calc(100% + 20px)",
            marginLeft: "-10px",
            paddingBottom: "23px",
            borderRadius: "10px 10px 0 0"
        };
    }
    return {};
});

const isNormalWithActivityBg = computed(() => {
    return !isPresale.value && isSeckill.value != 1 && isDiscount.value != 1 && props.productType !== "exchange" && !!props.activityPcImage;
});

const isPresale = computed(() => {
    return presaleDetailState.value && Object.keys(presaleDetailState.value).length > 0;
});

const isSupplierProduct = computed(() => {
    return !!props.product?.suppliersId && Number(props.product.suppliersId) > 0;
});

// 预售商品：不允许询价弹层打开（避免外部状态/残留状态触发）
watch(
    () => isPresale.value,
    (val) => {
        if (val) showEnquiry.value = false;
    },
    { immediate: true }
);

// 预售价格（定金预售显示定金；全款预售显示预售价）
const presalePriceValue = computed(() => {
    if (!isPresale.value) return "";
    if (presaleDetailState.value.saleMode === 2) return presaleDetailState.value.depositAmount || "";
    return presaleDetailState.value.presalePrice || productPrice.value || "";
});

// 预售总价（用于展示「预售价」：优先使用后端预售价字段，否则为定金 + 尾款）
const presaleTotalPrice = computed(() => {
    if (!isPresale.value) return "";
    const info: any = presaleDetailState.value || {};
    if (info.presalePrice) return info.presalePrice;
    const deposit = Number(info.depositAmount || 0);
    const finalAmount = Number(info.finalPaymentAmount || 0);
    const total = deposit + finalAmount;
    return total || "";
});

// 尾款支付时间显示
const finalPaymentTimeDisplay = computed(() => {
    if (!isPresale.value) return "";
    const info = presaleDetailState.value;
    if (info.finalPaymentStartTimeText) return info.finalPaymentStartTimeText;
    if (info.finalPaymentType === 1) {
        const days = info.finalPaymentRelativeDays ? `${info.finalPaymentRelativeDays}${t("天")}` : "";
        const durationDays = info.finalPaymentDurationDays ? `${info.finalPaymentDurationDays}${t("天")}` : "";
        const durationHours = info.finalPaymentDurationHours ? `${info.finalPaymentDurationHours}${t("小时")}` : "";
        return `${t("付定金后")}${days}${t("开始支付尾款，并需要在")}${durationDays}${durationHours}${t("内完成支付")}`;
    }
    if (info.finalPaymentStartTime && info.finalPaymentEndTime) {
        return `${info.finalPaymentStartTime}-${info.finalPaymentEndTime}`;
    }
    return "";
});

// 发货时间显示（全款预售/尾款后）
const formatDeliveryTime = computed(() => {
    if (!isPresale.value) return "";
    const info = presaleDetailState.value;
    if (info.deliveryType === 1) {
        const days = info.deliveryRelativeDays ? `${info.deliveryRelativeDays}${t("天")}` : "";
        const hours = info.deliveryRelativeHours ? `${info.deliveryRelativeHours}${t("小时")}` : "";
        return info.saleMode === 2 ? `${t("付尾款")}${days}${hours}${t("后")}${t("发货")}` : `${t("付全款")}${days}${hours}${t("后")}`;
    }
    return `${info.deliveryDate ?? ""}${t("开始")}`;
});

const onProductSkuChange = (item: any) => {
    if (item !== null) {
        skuId.value = item.skuId;
        const detailProductType = Number(props.product?.productType);
        // 预约商品先用规格库存；普通商品等可用性接口返回后再更新
        if (props.productType == "product" && (detailProductType === 5 || detailProductType === 6)) {
            productStock.value = item.skuStock;
        }
    }
    if (props.productType == "product") {
        loadPrice();
    }
};

const attrIds = ref("");
const handleAttrChange = (value: string) => {
    attrIds.value = value;
    loadPrice();
};

const loadPrice = async () => {
    try {
        const result = await getProductSkuDetail(props.id, {
            skuId: skuId.value,
            extraAttrIds: attrIds.value,
            shopId: props.product?.shopId
        });
        productPrice.value = result.price;
        const skuMarketPrice = result?.sku?.marketPrice ?? result?.marketPrice;
        if (skuMarketPrice !== undefined && skuMarketPrice !== null && skuMarketPrice !== "") {
            props.product.marketPrice = String(skuMarketPrice);
        }
        productStock.value = result.stock;
        // 预售商品：SKU变化时同步更新定金/尾款金额（后端若返回）
        if (isPresale.value) {
            if (result.depositAmount !== undefined) presaleDetailState.value.depositAmount = result.depositAmount;
            if (result.finalPaymentAmount !== undefined) presaleDetailState.value.finalPaymentAmount = result.finalPaymentAmount;
        }
        // 过滤抽奖活动
        result.promotion = result.promotion?.filter((item) => item.type !== 8);
        // 是否存在营销活动
        if (result.promotion.length > 0) {
            promotionManage(result.promotion);
        }

        if (isSeckill.value && maxProductNumber.value > 0) {
            productNumber.value = productNumber.value > maxProductNumber.value ? maxProductNumber.value : productNumber.value;
        } else if (productStock.value === 0) {
            productNumber.value = 1;
        } else {
            productNumber.value = productNumber.value > productStock.value ? productStock.value : productNumber.value;
        }
    } catch (error) {
    } finally {
    }
};

// 营销活动管理
const promotionManage = (promotion: SkuPromotion[]) => {
    // 排除秒杀 & 限时折扣
    promotionList.value = promotion.filter((item: any) => item.type !== 1 && item.type !== 6);
    const seckillData = promotion.find((item) => item.type === 1);
    const discountData = promotion.find((item) => item.type === 6);

    // 判断限时折扣存在
    if (discountData) {
        isDiscount.value = 1;
        discountValue.value.discountEndTime = discountData.data.item.endTime!;
        discountValue.value.discountPrice = discountData.data.item.discountPrice!;
        discountValue.value.canJoin = discountData.canJoin !== false;
        discountValue.value.restrictTip = discountData.restrictTip || "";
    } else {
        isDiscount.value = 0;
        discountValue.value = {
            discountEndTime: "",
            discountPrice: 0,
            canJoin: true,
            restrictTip: ""
        };
    }

    // 判断秒杀存在
    if (seckillData) {
        isSeckill.value = 1;
        seckillValue.value.seckillEndTime = seckillData.data.seckillEndTime!;
        seckillValue.value.seckillSales = seckillData.data.item.seckillSales!;
        seckillValue.value.seckillStock = seckillData.data.item.seckillStock!;
        seckillValue.value.canJoin = seckillData.canJoin !== false;
        seckillValue.value.restrictTip = seckillData.restrictTip || "";
        maxProductNumber.value = seckillData.data.seckillLimitNum!;
        productStock.value = seckillData.data.item.seckillStock;
    } else {
        isSeckill.value = 0;
        seckillValue.value = {
            seckillEndTime: "",
            seckillSales: 0,
            seckillStock: 0,
            canJoin: true,
            restrictTip: ""
        };
    }
};

// 二期会员标签（D1096）：秒杀 / 限时折扣不在人群范围时置灰购买入口，秒杀优先
const memberRestricted = computed(() => {
    if (isSeckill.value == 1) return seckillValue.value.canJoin === false;
    if (isDiscount.value == 1) return discountValue.value.canJoin === false;
    return false;
});

const memberRestrictTip = computed(() => {
    const tip = isSeckill.value == 1 ? seckillValue.value.restrictTip : discountValue.value.restrictTip;
    return tip || t("该商品仅限指定会员购买");
});

const init = () => {
    if (props.skuList.length === 0) {
        loadPrice();
    }
    maxProductNumber.value = 0;
};

init();

// b2b
const tabIndex = ref(0);
const isBatch = computed(() => {
    return isB2B() && tabIndex.value === 1;
});
</script>
<style lang="less" scoped>
.overseas-buy-actions() {
    display: flex;
    justify-content: space-between;
    padding-bottom: 25px;
    gap: 10px;

    .buy-btn {
        height: 48px;
        padding: 0 20px;
        border-radius: 4px;
        border: 1px solid;
        width: 100%;
        font-size: 16px;

        :deep(span) {
            font-weight: 700;
        }

        &:not(:disabled):not(.is-disabled):hover {
            opacity: 0.9;
        }
    }

    // 禁用态对齐 uni 端：沿用可用态皮肤，只做整体淡化。
    .buy-btn.is-disabled {
        --el-button-disabled-text-color: var(--el-button-text-color);
        --el-button-disabled-bg-color: var(--el-button-bg-color);
        --el-button-disabled-border-color: var(--el-button-border-color);
    }

    .buy-btn:disabled,
    .buy-btn.is-disabled {
        opacity: 0.5;
    }

    .disabled .btn {
        color: #ccc;
        background-color: #f7f7f7;
        border: 1px solid #ebebeb;
        cursor: not-allowed;

        &:hover {
            opacity: 0.9;
        }
    }

    .buynow {
        height: 48px;
        padding: 0 20px;
    }
}

.product_info {
    .active-price-info .price-group,
    .main-price-box .price1,
    .main-price-box .market_price {
        :deep(.price) {
            align-items: baseline;
            vertical-align: baseline;
        }
    }

    // O2O 批发态下统一收紧阶梯价、门店卡片和规格区之间的外沿间距；
    // 运行时条件同时限定批发态与门店版，不改变非 O2O 或零售态布局。
    &.o2o-wholesale-compact {
        .actions {
            overflow: visible;
        }

        .item.main-price-box {
            padding-bottom: 0;
        }

        :deep(.wholesale-ladder-bar .ladder-scroll) {
            padding-bottom: 0;
        }

        // 简约风所有单档价格取消公共等高占位，并补齐顶部 8px；
        // 与上方 16px 共同形成 24px 模块间距，区间价和单价 + 起批量保持一致。
        :deep(.wholesale-price-range) {
            min-height: 0;
            align-items: flex-start;
            padding-top: 8px;
        }

        :deep(.product-store-card .store-item-wrapper) {
            margin-top: 24px;
            margin-bottom: 24px;
        }

        :deep(.wholesale-sku-selector > .dim-group:first-child) {
            padding-top: 0;
        }
    }

    // 非 O2O 环境没有门店卡承接模块间距：价格行底部 16px + 首规格顶部 8px = 24px。
    &.wholesale-no-store-spacing {
        // 多档阶梯保留顶部 8px，清掉底部 8px，避免与价格行 16px、首规格 8px 叠成 32px。
        :deep(.wholesale-ladder-bar .ladder-scroll) {
            padding-bottom: 0;
        }

        // 上方信息行底部 16px + 单档价格顶部 8px = 24px；区间价和普通单档价一致。
        :deep(.wholesale-price-range) {
            min-height: 0;
            align-items: flex-start;
            padding-top: 8px;
        }

        :deep(.wholesale-sku-selector > .dim-group:first-child) {
            padding-top: 8px;
        }
    }

    .active-box {
        // background: url("/assets/images/product/pro-time-bg.png") no-repeat;
        background-size: 100% 100%;
        padding: 13px 0;
        border-radius: 6px 6px 0 0;
        font-size: 16px;
        position: relative;
        &.has-activity-bg {
            padding: 13px 10px;
            display: flex;
            .countdown {
                color: #fff !important;
            }
            .active-time {
                margin-left: auto;
            }
        }

        &.has-activity-bg::after {
            content: "";
            position: absolute;
            left: -1px;
            right: -1px;
            bottom: -4px;
            height: 16px;
            background: #fff;
            border-radius: 12px 12px 0 0;
        }

        .active-ico {
            font-weight: bold;
            font-size: 16px;
            color: rgb(255, 255, 255);
            line-height: 24px;
            border-radius: 4px;
            background-color: rgb(227, 114, 0);
            padding: 0px 6px;
            margin-right: 8px;
            i {
                margin-right: 5px;
                font-size: 18px;
            }
        }
        .active-time {
            margin-right: 10px;
            color: #fff;
        }

        &.active-box-normal {
            .active-price-info {
                display: flex;
                align-items: baseline;
                gap: 24px;
                margin-left: 10px;
                color: #fff;

                .price-group {
                    display: flex;
                    align-items: baseline;
                    gap: 4px;
                    font-size: 14px;

                    :deep(.price) {
                        color: #fff;
                        font-size: 22px;

                        .util {
                            color: #fff;
                            font-size: 12px;
                        }
                        .num {
                            color: #fff;
                        }
                    }

                    &.market {
                        :deep(.price) {
                            color: rgba(255, 255, 255, 0.7);
                            font-size: 14px;
                            text-decoration: line-through;

                            .util,
                            .num {
                                color: rgba(255, 255, 255, 0.7);
                            }
                        }
                    }
                }
            }
        }
    }

    .product_name {
        .name {
            .presale-text {
                display: inline-block;
                font-weight: bold;
                font-size: 16px;
                color: #ffffff;
                line-height: 24px;
                border-radius: 4px;
                background-color: #e37200;
                padding: 0px 6px;
                margin-right: 6px;
            }
        }
    }

    .presale-basic {
        .presale-price-box {
            display: flex;
            flex-direction: column;
            gap: 8px;
            width: 100%;
            align-items: stretch;

            .presale-line {
                display: flex;
                align-items: baseline;
                column-gap: 10px;
            }
        }
    }

    // 尾款和流程样式（独立定义，支持在已售区域展示）
    .presale-final {
        flex-direction: column;
        align-items: stretch;
        row-gap: 4px;

        .presale-final-grid {
            display: grid;
            grid-template-columns: max-content 1fr;
            column-gap: 10px;
            row-gap: 4px;
            align-items: baseline;
            min-width: 0;
        }
        .presale-time {
            font-size: 12px;
            color: #888b94;
            white-space: normal;
            word-break: break-word;
            line-height: 1.4;
            grid-column: 2;
            align-self: start;
        }
    }

    .presale-process {
        align-items: center;
        column-gap: 10px;
        flex-wrap: wrap;
        .process-text {
            font-size: 14px;
            color: #1a1a1a;
        }
        .presale-info-icon {
            color: #888b94;
            cursor: pointer;
            font-size: 14px;
        }
    }

    .presale-rule-content {
        .rule-section {
            margin-bottom: 16px;
            &:last-child {
                margin-bottom: 0;
            }
            .rule-title {
                font-size: 16px;
                font-weight: 600;
                color: #333;
                margin-bottom: 12px;
            }
            .rule-item {
                font-size: 14px;
                color: #666;
                line-height: 1.8;
                margin-bottom: 8px;
                display: flex;
                &:last-child {
                    margin-bottom: 0;
                }
                .rule-number {
                    flex-shrink: 0;
                    margin-right: 4px;
                }
            }
        }
    }

    .basic {
        .item {
            display: flex;
            align-items: center;
            padding-bottom: 16px;

            &.main-price-box {
                display: flex;
                align-items: baseline;
                column-gap: 9px;
            }

            .tit {
                color: #999;
                min-width: 36px;
            }

            .tit2 {
                color: #999;
            }

            .market_price {
                color: #888;
                text-decoration: line-through;
            }

            .price1 {
                color: #1a1a1a;

                &.theme {
                    color: var(--general);
                }

                // 批发态：阶梯条占满本行剩余宽度并允许内部溢出，使阶梯表能检测到超宽并显示左右滚动箭头
                &.wholesale-price1 {
                    flex: 1;
                    min-width: 0;
                }
            }

            .evaluate {
                margin-left: 10px;
                font-size: 14px;
                color: #888b94;
            }

            .num {
                font-weight: normal;
                font-size: 14px;
                color: rgb(136, 139, 148);
                line-height: 14px;
            }

            .tips_i_box {
                display: flex;
                flex-wrap: wrap;
                width: 95%;

                span {
                    font-size: 14px;
                    display: flex;
                    align-items: center;
                    color: #888b94;
                }
            }
        }
        .exchange {
            display: flex;
            align-items: center;
            .tit {
                padding: 2px 5px;
                background: var(--main-bg);
                font-size: 14px;
                color: #fff;
                cursor: pointer;
                text-align: center;
                margin-right: 10px;
            }
            .price1 {
                margin-top: 5px;
                :deep(.price) {
                    color: var(--general);
                    font-size: 26px;
                    margin-right: 15px;

                    .util {
                        font-size: 12px;
                        line-height: 26px;
                    }
                    .num {
                        line-height: 18px;
                    }
                }
                .exchange-num {
                    font-size: 18px;
                    margin-right: 5px;
                }
            }
        }
    }

    // 覆盖通用 .basic .item 的居中对齐，保证尾款块左对齐展示
    .basic.bg {
        .item.presale-final {
            align-items: stretch;
        }
    }

    .actions {
        // border-top: 1px dotted #dddddd;
        overflow: hidden;
        // padding: 15px 1px 0 0;
        &.no-border {
            border-top: none;
        }

        .number-box {
            display: flex;
            margin-bottom: 16px;

            .title {
                margin-right: 6px;
                color: rgb(136, 139, 148);
                font-size: 14px;
                font-weight: 400;
                line-height: 36px;
            }

            .con {
                display: flex;
                align-items: center;
                :deep(.el-input-number) {
                    width: 120px;
                    border: none;
                }
                :deep(.el-input-number__decrease) {
                    border-right: none !important;
                }
                :deep(.el-input-number__increase) {
                    border-left: none !important;
                }
                :deep(.el-input__wrapper) {
                    box-shadow: none !important;
                    background: #f7f8fc;
                }
            }
        }

        // 二期会员标签：不可购买红字提示，置于数量选择下方
        .member-restrict-tip {
            display: flex;
            align-items: center;
            margin-bottom: 16px;
            color: #f54e4e;
            font-size: 13px;

            .iconfont-pc {
                margin-right: 4px;
                font-size: 14px;
            }
        }

        .add_cart {
            .overseas-buy-actions();
        }

        .link-code {
            display: flex;
            margin-left: 60px;
            margin-bottom: 30px;

            .link-code-title {
                display: flex;
                align-items: center;

                .link-code-phone {
                    width: 20px;
                    height: 20px;
                    background: url("/assets/images/product/phone_color.png") no-repeat center 0;
                    background-size: 100% 100%;
                }

                .link-code-font {
                    font-size: 12px;
                    color: var(--general);
                }
            }

            .link-code-main {
                display: none;
                cursor: pointer;
                width: 188px;
                height: 176px;
                margin-left: 140px;
                z-index: 9;
                float: left;
                position: absolute;
                background: url("/assets/images/product/code-border.png") no-repeat center 0;
                background-size: cover;

                .link-code-qrimg {
                    margin-top: 13px;
                    margin-left: 25px;
                    width: 150px;
                }
            }
        }

        .link-code:hover .link-code-main {
            display: block;
        }
    }
}
.external-buy-actions {
    .add_cart {
        .overseas-buy-actions();
    }
}
.product_name {
    .name {
        font-size: 18px;
        color: #000;
        font-weight: 700;
        font-family: "Inter,SF Pro Text,Roboto,Helvetica Neue,Helvetica,Tahoma,Arial,PingFang SC,Microsoft YaHei";
        padding-bottom: 16px;

        &.name--with-desc {
            padding-bottom: 11px;
        }
    }

    .desc {
        padding: 0 0 12px;
        color: #888b94;
        font-size: 12px;
        font-weight: normal;
    }
}
.serviceAction {
    :deep(.el-collapse-item__title) {
        font-weight: 700;
        font-size: 16px;
    }

    :deep(.el-collapse-item__content) {
        padding: 0;
    }
    .collapse-icon {
        svg {
            transition: transform 0.3s ease;
        }
        &.active {
            svg {
                transform: rotate(180deg);
            }
        }
    }
}
.addToCart {
    // 配色同时以 Element Plus 变量声明，禁用态才能沿用这套黑底白字而非 EP 的灰色
    --el-button-text-color: #fff;
    --el-button-bg-color: #1a1a1a;
    --el-button-border-color: #505259;
    --el-button-hover-text-color: #fff;
    --el-button-hover-bg-color: #333;
    --el-button-hover-border-color: #505259;
    --el-button-active-text-color: #fff;
    --el-button-active-bg-color: #111;
    --el-button-active-border-color: #505259;
    border: 1.5px solid #505259;
    color: #fff;
    background-color: #1a1a1a;

    &.tig-button:hover:not(:disabled),
    &:hover:not(:disabled):not(.is-disabled) {
        color: #fff;
        background-color: #333;
        border-color: #505259;
    }
}
.pg_16 {
    padding-bottom: 16px;
}
</style>
