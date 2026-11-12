<template>
    <div
        class="product_info"
        :class="{
            'new-ui': uiVersion === 'new',
            'standard-wholesale-spacing': isWholesale && uiVersion === 'new',
            'wholesale-no-store-spacing': isWholesale && uiVersion === 'new' && !isStore(),
            'has-product-brief': hasProductBrief
        }"
    >
        <!-- 预售商品头部 -->
        <template v-if="isPresale">
            <div class="active-box flex justify-between" :style="activeBoxStyle">
                <div class="active-ico flex align-end">
                    <el-icon :size="18"><AlarmClock /></el-icon>
                    <div class="text">{{ $t("预售") }}</div>
                </div>
            </div>
        </template>

        <div :class="['basic', !activityPcImage ? 'bg' : '']" v-if="productType == 'exchange'">
            <div class="item exchange" :class="[activityPcImage ? 'active-box' : '']" :style="activeBoxStyle">
                <div class="tit">{{ $t("{0}兑换", [commonStore.integralName]) }}</div>
                <div class="price1">
                    <FormatPrice v-model="exchangeDetail.discountsPrice"></FormatPrice>
                    <span class="red">+</span>
                    &nbsp;&nbsp;
                    <span class="exchange-num red">{{ exchangeDetail.exchangeIntegral }}</span>
                    <span class="red">{{ commonStore.integralName }}</span>
                </div>
            </div>
        </div>
        <template v-else>
            <template v-if="!isPresale && isSeckill == 1 && !isWholesale">
                <div class="active-box flex justify-between" :style="activeBoxStyle">
                    <div class="active-ico flex align-center">
                        <i class="iconfont-pc icon-miaosha"></i>
                        <span>{{ $t("秒杀") }}</span>
                    </div>
                    <div class="active-time">
                        <ClientOnly>
                            <Countdown :endTime="seckillValue.seckillEndTime"></Countdown>
                        </ClientOnly>
                    </div>
                </div>
            </template>
            <template v-if="!isPresale && isDiscount == 1 && !isWholesale">
                <div class="active-box flex justify-between" :style="activeBoxStyle">
                    <div class="active-ico flex align-center">
                        <i class="iconfont-pc icon-miaosha"></i>
                        <span>{{ $t("限时折扣") }}</span>
                    </div>
                    <div class="active-time">
                        <ClientOnly>
                            <Countdown :endTime="discountValue.discountEndTime"></Countdown>
                        </ClientOnly>
                    </div>
                </div>
            </template>
            <template v-if="isNormalWithActivityBg && !isWholesale">
                <div class="active-box active-box-normal flex align-center" :style="activeBoxStyle">
                    <div class="active-price-info flex align-center">
                        <div class="price-group" v-if="productPrice !== ''">
                            <span>{{ $t("本站价") }}：</span>
                            <FormatPrice v-model="productPrice"></FormatPrice>
                        </div>
                        <div
                            class="price-group market"
                            v-if="product.marketPrice && Number(product.marketPrice) > 0 && (commonStore.showMarketprice || isSupplierProduct)"
                        >
                            <FormatPrice v-model="product.marketPrice" :font-style="{ 'text-decoration': 'line-through' }"></FormatPrice>
                        </div>
                    </div>
                </div>
            </template>
        </template>

        <!-- 预售商品价格信息 -->
        <div class="presalePrice" v-if="isPresale && productType != 'exchange'">
            <div class="presale-info-content">
                <div class="presalePriceLine deposit-line" v-if="presaleDetail.saleMode === 2">
                    <span class="price-label">{{ $t("定金") }}：</span>
                    <FormatPrice
                        :decimalsStyle="{ fontSize: '14px' }"
                        :fontStyle="{ fontSize: '16px' }"
                        :currencyStyle="{ fontSize: '12px' }"
                        currency-alignment="bottom"
                        class="priceBox"
                        :model-value="presalePriceValue"
                    ></FormatPrice>
                </div>
                <div class="presalePriceLine presale-price-line" v-if="presaleDetail.saleMode === 2" style="margin-top: -10px">
                    <span class="price-label">{{ $t("预售价") }}：</span>
                    <FormatPrice
                        :decimalsStyle="{ fontSize: '14px' }"
                        :fontStyle="{ fontSize: '24px' }"
                        :currencyStyle="{ fontSize: '14px' }"
                        currency-alignment="bottom"
                        class="priceBox"
                        :model-value="product.productPrice || 0"
                    ></FormatPrice>
                </div>

                <div class="presalePriceLine deposit-line" v-else>
                    <span class="price-label">{{ $t("预售价") }}：</span>
                    <FormatPrice
                        :decimalsStyle="{ fontSize: '14px' }"
                        :fontStyle="{ fontSize: '16px' }"
                        :currencyStyle="{ fontSize: '12px' }"
                        currency-alignment="bottom"
                        class="priceBox"
                        :model-value="presalePriceValue"
                    ></FormatPrice>
                </div>

                <div class="presalePriceLine presale-price-line" v-if="presalePromotionList.length > 0">
                    <span class="price-label">{{ $t("优惠活动") }}：</span>
                    <ProductCouponList :promotionList="presalePromotionList" :productId="product.productId" :shopId="product.shopId ?? 0"> </ProductCouponList>
                </div>
            </div>
        </div>

        <div
            class="basic"
            :class="{ bg: !isWholesale, 'basic-wholesale': isWholesale, isActive: !isWholesale && (isSeckill == 1 || isDiscount == 1 || isNormalWithActivityBg) }"
            v-if="showBasicInfoSection"
        >
            <div class="item" v-if="showMarketPriceInfo">
                <div class="tit">{{ $t("市场价") }}：</div>
                <div class="market_price">
                    <FormatPrice v-model="product.marketPrice" :font-style="{ 'text-decoration': 'line-through' }"></FormatPrice>
                </div>
            </div>
            <div class="item" :class="{ 'item-wholesale': isWholesale }" v-if="showProductPriceInfo || hasRankDetail">
                <template v-if="showProductPriceInfo">
                    <div class="tit" v-if="!isWholesale">{{ isSeckill == 1 ? $t("秒杀价") : $t("本站价") }}：</div>
                    <div class="price1" :class="{ 'price1-wholesale': isWholesale }" v-if="productPrice !== '' || isWholesale">
                        <FormatPrice v-if="!isWholesale" v-model="productPrice"></FormatPrice>
                    </div>
                </template>
                <div class="rank-info" v-if="hasRankDetail">
                    <div class="tit2">{{ $t("评价") }}：</div>
                    <div class="star">
                        <span class="icon-star" :class="`icon-star-${rankDetail.averageRank}`"></span>
                    </div>
                    <div class="evaluate">
                        {{ $t("已有") }} <span class="font-color"> {{ rankDetail.total }}</span> {{ $t("人评价") }}，{{ rankDetail.goodPercent }}% {{ $t("好评") }}
                    </div>
                </div>
            </div>
        <!-- 旧版 UI：优惠活动内联展示（带标签） -->
        <div class="item" v-if="showPromotionInfo && uiVersion !== 'new' && !isWholesale">
                <ProductCouponList :promotionList="promotionList" :productId="product.productId" :shopId="product.shopId ?? 0">
                    <div class="tit">{{ $t("优惠活动") }}：</div></ProductCouponList
                >
            </div>
        </div>
        <!-- 新版 UI：优惠活动标签，展示在基础信息下方 -->
        <div class="promotion-tags" v-if="uiVersion === 'new' && showBasicInfoSection && showPromotionInfo && !isWholesale">
            <ProductCouponList :promotionList="promotionList" :productId="product.productId" :shopId="product.shopId ?? 0" :uiVersion="uiVersion"></ProductCouponList>
        </div>

        <!-- 预售商品销量 -->
        <div v-if="isPresale && product.virtualSales !== undefined" class="presale-sales">
            <span>{{ $t("销量") }}：</span>
            <span>{{ product.virtualSales }} {{ $t("件") }}</span>
        </div>
        <div v-if="isPresale && product.virtualSales !== undefined" class="presale-sales-divider"></div>

        <!-- 预售尾款信息 -->
        <div class="presale-final-payment" v-if="isPresale && presaleDetail.saleMode === 2">
            <div class="final-payment-content">
                <span class="final-payment-label">{{ $t("尾款") }}：</span>
                <div class="final-payment-amount-box">
                    <FormatPrice
                        :decimalsStyle="{ fontSize: '14px', alignSelf: 'self-end' }"
                        :fontStyle="{ fontSize: '14px', alignSelf: 'self-end' }"
                        class="final-payment-amount"
                        :model-value="presaleDetail?.finalPaymentAmount || 0"
                    ></FormatPrice>
                    <span v-if="finalPaymentTimeDisplay" class="final-payment-time"> ({{ finalPaymentTimeDisplay }}) </span>
                </div>
            </div>
        </div>

        <!-- 预售流程和发货时间 -->
        <div class="presale-process" v-if="isPresale && presaleDetail.saleMode === 2">
            <span class="process-label">{{ $t("流程") }}：</span>
            <span class="process-text">1.{{ $t("付定金") }}-2.{{ $t("付尾款") }}-3.{{ $t("发货") }}</span>
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
        <div class="presale-process" v-else-if="isPresale">
            <span class="process-label">{{ $t("发货") }}：</span>
            <span class="process-text">{{ deliveryTimeTextForFullPresale }}</span>
        </div>

        <div class="basic-info" v-if="!isPresale">
            <div class="item" v-if="commonStore.showSelledCount">
                <div class="tit">{{ $t("销量") }}：</div>
                <div class="num">{{ (isSeckill == 1 ? seckillValue.seckillSales : product.virtualSales) ?? 0 }} {{ $t("件") }}</div>
            </div>
            <template v-if="product.productType === 7 || product.productType === 8">
                <div class="item">
                    <div class="tit">{{ $t("说明") }}：</div>
                    <div class="num">{{ $t("此商品不支持售后，如需售后请联系客服") }}</div>
                </div>
            </template>
            <template v-if="serviceList.length > 0 && uiVersion !== 'new'">
                <div class="item align-start">
                    <div class="tit">{{ $t("服务") }}：</div>
                    <div class="tips_i_box">
                        <template v-for="(item, index) in serviceList" :key="index">
                            <!-- <template v-if="item.icoImg"> -->
                            <span>
                                <template v-if="item.icoImg"> <img :src="imageFormat(item.icoImg)" width="15" height="15" />&nbsp; </template>
                                {{ item.productServiceName }}
                            </span>
                            <!-- </template> -->
                        </template>
                    </div>
                </div>
            </template>
        </div>


        <template v-if="isB2B() && skuList.length > 1 && commonStore.bulkPurchase === 1 && product.productType !== 4 && !isWholesale">
            <ProductB2bTab v-model:tabIndex="tabIndex" :uiVersion="uiVersion"></ProductB2bTab>
        </template>

        <div class="actions">
            <template v-if="!isWholesale">
            <template v-if="tabIndex !== 1">
                <template v-if="skuList.length > 0">
                    <ProductSku
                        v-model="attrList"
                        :skuList="skuList"
                        :checkedValue="checkedValue"
                        :productType="productType"
                        :uiVersion="uiVersion"
                        @change="onProductSkuChange"
                    >
                    </ProductSku>
                </template>
                <template v-if="!(product.productType === 4 && product.isBuy === 1)">
                    <!-- 新版 UI：数量标题置于上方，步进器与库存状态同行 -->
                     <div class="split-line" v-if="uiVersion === 'new' && skuList.length == 0"></div>
                    <div class="number-box" v-if="uiVersion === 'new'">
                        <div class="title">{{ $t("数量") }}</div>
                        <div class="con">
                            <InputNumber
                                v-model="productNumber"
                                :rounded="true"
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
                    <!-- 旧版 UI -->
                    <div class="number-box" v-else>
                        <div class="title">{{ $t("数量") }}：</div>
                        <div class="con">
                            <InputNumber
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
                    :uiVersion="uiVersion"
                ></ProductB2bSku>
            </template>
            <ProductAttr v-model="attrList" :uiVersion="uiVersion" @change="handleAttrChange"> </ProductAttr>
            <template v-if="isBatch && productAmount && productAmount.count > 0">
                <ProductAmount :select-skus="selectSkus" :product-amount="productAmount" :uiVersion="uiVersion"></ProductAmount>
            </template>
            <!-- 新版 UI 下购买操作统一由悬浮购买栏(ProductFloatBuyBar)承载，隐藏此处冗余按钮；B2B 批量采购模式无悬浮栏，仍保留 -->
            <template v-if="showInlineBuy">
            <template v-if="product.productType === 4 && product.isBuy === 1">
                <div class="add_cart">
                    <el-button :disabled="true" class="buy-btn buynow" type="primary">{{ $t("商品已购买") }}</el-button>
                </div>
            </template>
            <template v-else>
                <div class="add_cart">
                    <template v-if="isBatch">
                        <ProductBuy
                            :shop-id="product.shopId"
                            :extraAttrIds="attrIds"
                            :id="product.productId"
                            :skuItem="skuItem"
                            :disabled="skuItem.length === 0"
                            :isQuick="true"
                        >
                            <el-button :disabled="skuItem.length === 0" class="buy-btn buynow" type="primary">{{ $t("立即购买") }}</el-button>
                        </ProductBuy>
                        <ProductBuy
                            :shop-id="product.shopId"
                            :extraAttrIds="attrIds"
                            :id="product.productId"
                            :disabled="skuItem.length === 0"
                            :skuItem="skuItem"
                        >
                            <el-button :disabled="skuItem.length === 0" class="buy-btn buynow">{{ $t("批量加入购物车") }}</el-button>
                        </ProductBuy>
                    </template>
                    <template v-else>
                        <!-- 预售商品购买按钮 -->
                        <template v-if="isPresale && productType == 'product'">
                            <div class="add_cart presale-cart">
                                <ProductBuy
                                    :shop-id="product.shopId"
                                    :extraAttrIds="attrIds"
                                    :id="product.productId"
                                    :skuId="skuId"
                                    :disabled="isOutOfStock"
                                    :number="productNumber"
                                    :isQuick="true"
                                    v-if="presaleDetail.saleMode === 2"
                                >
                                    <el-button
                                        class="buy-btn buynow presaleBuy"
                                        type="primary"
                                        :style="{ backgroundColor: 'var(--general)', borderColor: 'var(--general)' }"
                                        :disabled="isOutOfStock"
                                    >
                                        <FormatPrice
                                            :model-value="presalePriceValue"
                                            :fontStyle="{ fontSize: '18px', fontWeight: '600', color: '#fff' }"
                                        ></FormatPrice>
                                        <span style="color: #fff">{{ $t("立即预定") }}</span>
                                    </el-button>
                                </ProductBuy>
                                <ProductBuy
                                    :shop-id="product.shopId"
                                    :extraAttrIds="attrIds"
                                    :id="product.productId"
                                    :skuId="skuId"
                                    :disabled="isOutOfStock"
                                    :number="productNumber"
                                    :isQuick="true"
                                    v-else
                                >
                                    <el-button
                                        class="buy-btn buynow presaleBuy"
                                        type="primary"
                                        :style="{ backgroundColor: 'var(--general)', borderColor: 'var(--general)' }"
                                        :disabled="isOutOfStock"
                                    >
                                        <span style="color: #fff">{{ $t("立即预购") }}</span>
                                    </el-button>
                                </ProductBuy>
                            </div>
                        </template>

                        <template v-else-if="productType == 'product'">
                            <template v-if="memberRestricted">
                                <!-- 二期会员标签：不在人群范围，按钮置灰，文案与可购买态保持一致 -->
                                <el-button disabled class="buy-btn" v-if="isSeckill == 1 && isOutOfStock">{{ $t("已抢完") }}</el-button>
                                <el-button disabled class="buy-btn" v-else-if="product.productType === 1 && !isCycleInfo">{{ $t("加入购物车") }}</el-button>
                                <el-button disabled class="buy-btn buynow" type="primary">{{
                                    $t(isReservationProduct ? "立即预约" : "立即购买")
                                }}</el-button>
                            </template>
                            <template v-else>
                                <ProductBuy
                                    :shop-id="product.shopId"
                                    :extraAttrIds="attrIds"
                                    :id="product.productId"
                                    :skuId="skuId"
                                    :disabled="isOutOfStock"
                                    :number="productNumber"
                                >
                                    <el-button :disabled="isOutOfStock" class="buy-btn" v-if="isSeckill == 1 && isOutOfStock">{{ $t("已抢完") }}</el-button>
                                    <el-button :disabled="isOutOfStock" class="buy-btn" v-else-if="product.productType === 1 && !isCycleInfo">{{
                                        $t("加入购物车")
                                    }}</el-button>
                                </ProductBuy>

                                <template v-if="isOutOfStock && !isReservationProduct">
                                    <el-button class="buy-btn buynow" type="primary" @click="$router.push('/search')">{{ $t("查看其他商品") }}</el-button>
                                </template>
                                <template v-else>
                                    <ProductBuy
                                        :extraAttrIds="attrIds"
                                        :id="product.productId"
                                        :skuId="skuId"
                                        :disabled="disableQuickBuy"
                                        :number="productNumber"
                                        :isQuick="true"
                                        :shop-id="product.shopId"
                                        :isCycleInfo="isCycleInfo"
                                    >
                                        <el-button :disabled="disableQuickBuy" class="buy-btn buynow" type="primary">{{
                                            $t(product.productType === 5 || product.productType === 6 ? "立即预约" : "立即购买")
                                        }}</el-button>
                                    </ProductBuy>
                                </template>
                            </template>
                        </template>
                        <template v-if="productType == 'exchange'">
                            <ProductBuy
                                :extraAttrIds="attrIds"
                                :id="exchangeDetail.id"
                                :skuId="skuId"
                                :disabled="isOutOfStock"
                                :number="productNumber"
                                :productType="productType"
                                :isQuick="true"
                                :shop-id="product.shopId"
                            >
                                <el-button class="buy-btn buynow" type="primary" :disabled="isOutOfStock">{{ $t("立即兑换") }}</el-button>
                            </ProductBuy>
                        </template>
                    </template>
                    <!-- 立即询价：会员限制时同步置灰，缺货时仍可询价 -->
                    <template v-if="commonStore.isEnquiry === 1 && (isB2B() || isOverseas() || isS2b2c())">
                        <el-button
                            class="buy-btn buynow"
                            :disabled="productType == 'product' && memberRestricted"
                            :title="productType == 'product' && memberRestricted ? memberRestrictTip : ''"
                            @click="showEnquiry = !showEnquiry"
                            >{{ $t("立即询价") }}</el-button
                        >
                    </template>
                </div>
            </template>
            </template>
            </template>
        </div>
        <!-- 新版 UI：悬浮购买栏 + 底部占位（购买操作由悬浮栏承载；B2B 批量采购模式承载批量加购/立即购买） -->
        <template v-if="uiVersion === 'new'">
            <ProductFloatBuyBar
                :product="product"
                :productType="productType"
                :skuId="skuId"
                :number="productNumber"
                :productStock="productStock"
                :extraAttrIds="attrIds"
                :isSeckill="isSeckill"
                :memberRestricted="productType == 'product' && memberRestricted"
                :restrictTip="memberRestrictTip"
                :isPresale="isPresale"
                :presaleDetail="presaleDetail"
                :presalePriceValue="presalePriceValue"
                :exchangeDetail="exchangeDetail"
                :shopId="product.shopId"
                :isCycleInfo="isCycleInfo"
                :isBatch="isBatch"
                :skuItem="skuItem"
                :isWholesale="isWholesale"
                :disabled="product.productStatus === 0"
                :wholesaleLoadingAdd="wholesaleLoadingAdd"
                :wholesaleLoadingBuy="wholesaleLoadingBuy"
                @enquiry="showEnquiry = !showEnquiry"
                @add-to-cart="onWholesaleAddToCart"
                @buy-now="onWholesaleBuyNow"
            ></ProductFloatBuyBar>
        </template>
        <!-- 新版 UI：服务说明折叠面板（服务名称 + 服务描述） -->
        <template v-if="uiVersion === 'new' && serviceList && serviceList.length > 0">
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
                            <div class="service-desc">
                                <span>{{ item.productServiceDesc || "" }} </span>
                            </div>
                        </el-collapse-item>
                    </template>
                </el-collapse>
            </div>
        </template>
        <div v-if="uiVersion === 'new'" style="height: 100px"></div>
        <ProductEnquiry v-model="showEnquiry" :productId="product.productId"></ProductEnquiry>
    </div>
</template>
<script lang="ts" setup>
import { reactive, ref, onMounted, watch, computed } from "vue";
import { InputNumber, Countdown } from "@/components/product";
import { SelectRegion } from "@/components/region";
import { getProductSkuDetail, getPromotion } from "~/api/product/product";
import { imageFormat, formatDistance } from "@/utils/format";
import type { ProductItem, AttrList, SkuList, ServiceList, SkuPromotion, ExchangeDetail, ProductAmountItem } from "~/types/product/product.d";
import { useCommonStore } from "~/store/common";
import { AlarmClock } from "@element-plus/icons-vue";

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
    productStock: {
        type: Number,
        default: 0
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
    // 门店信息
    storeInfo: {
        type: Object,
        default: {}
    },
    isCycleInfo: {
        type: Boolean,
        default: false
    },
    // UI 版本：old 老版，new 新版
    uiVersion: {
        type: String,
        default: "old"
    },
    presaleDetail: {
        type: Object,
        default: () => ({})
    },
    activityPcImage: {
        type: String as PropType<string | null>,
        default: null
    },
    // 批发态总开关（由 Product.vue 传入）：true 时价格/规格/购买区切换为批发形态并去营销化。
    // 该 prop 始终存在（非批发态为 false），以便零售模板用 v-if="!isWholesale" 兜底。
    isWholesale: {
        type: Boolean,
        default: false
    },
    // 标准版标题区的商品描述自带 12px 下内边距，用于避免与批发价格区的顶部间距叠加。
    hasProductBrief: {
        type: Boolean,
        default: false
    },
    // 详情接口下发的批发阶梯价表；常驻 prop，非批发态为 null
    wholesalePriceList: {
        type: Array as PropType<any[] | null>,
        default: null
    }
});
const attrList = ref<object>(props.attrList);
const exchangeDetail = ref<ExchangeDetail>(props.exchangeDetail);
const productPrice = ref<any>(props.productPrice);
const productStock = ref<number>(props.productStock);
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
    // 二期会员标签（D1096）：false 时秒杀正常展示但置灰购买并提示
    canJoin: true,
    restrictTip: ""
});
const discountValue = ref({
    discountEndTime: "",
    discountPrice: 0,
    // false 时不享受限时折扣，按普通价格购买
    canJoin: true,
    restrictTip: ""
});
const isOutOfStock = computed(() => {
    const stock = Number(productStock.value);
    return !Number.isFinite(stock) || stock <= 0;
});
const isReservationProduct = computed(() => props.product?.productType === 5 || props.product?.productType === 6);
const skuPriceLoading = ref(false);
const disableQuickBuy = computed(() => isOutOfStock.value || skuPriceLoading.value);
let latestLoadPriceRequestId = 0;
const productAmount = ref<ProductAmountItem>({
    count: 0,
    total: "0"
});
const selectSkus = ref<any[]>([]);
const skuItem = ref<any[]>([]);
const showEnquiry = ref(false);
const activeName = ref("");
const commonStore = useCommonStore();
const route = useRoute();
const router = useRouter();
const { t } = useI18n();

const activeBoxStyle = computed(() => {
    if (props.activityPcImage) {
        return {
            background: `url(${imageFormat(props.activityPcImage)}) no-repeat center center`,
            backgroundSize: "cover"
        };
    }
    return {};
});

const isNormalWithActivityBg = computed(() => {
    return !isPresale.value && isSeckill.value != 1 && isDiscount.value != 1 && props.productType !== "exchange" && !!props.activityPcImage;
});

// 秒杀属于活动资格购买；限时折扣不满足资格时仍可按普通价格购买
const memberRestricted = computed(() => {
    if (isSeckill.value == 1) return seckillValue.value.canJoin === false;
    return false;
});

const memberRestrictTip = computed(() => {
    const tip = isSeckill.value == 1 ? seckillValue.value.restrictTip : discountValue.value.restrictTip;
    return tip || t("该商品仅限指定会员购买");
});

const isPresale = computed(() => {
    return props.presaleDetail && typeof props.presaleDetail === "object" && Object.keys(props.presaleDetail).length > 0;
});

const isSupplierProduct = computed(() => {
    return !!props.product?.suppliersId && Number(props.product.suppliersId) > 0;
});

// 评价模块只有在存在评价总数时才展示，避免渲染空占位。
// D1146：评价功能关闭时一并收敛，L126 的容器判断复用同一入口。
const hasRankDetail = computed(() => commonStore.productCommentEnabled === 1 && Number(props.rankDetail?.total || 0) > 0);

// 市场价需要满足业务开关、当前价存在且市场价本身有效，才显示这一行。
// 供应商商品：有市场价则展示划线价，无市场价则不展示（不受全局开关约束）。
// 批发态：与标准版一致，不展示市场价。
const showMarketPriceInfo = computed(() => {
    return (
        !props.isWholesale &&
        !isNormalWithActivityBg.value &&
        isSeckill.value != 1 &&
        productPrice.value !== "" &&
        (commonStore.showMarketprice || isSupplierProduct.value) &&
        !!props.product?.marketPrice &&
        Number(props.product.marketPrice) > 0
    );
});

const showProductPriceInfo = computed(() => {
    // 批发态：阶梯价不依赖零售 productPrice。多规格批发不挂零售 SKU 选择器，不会触发 loadPrice，
    if (props.isWholesale) return !isNormalWithActivityBg.value;
    return !isNormalWithActivityBg.value && productPrice.value !== "";
});

const showPromotionInfo = computed(() => {
    return promotionList.value.length > 0;
});

// 这块基础信息区域只在内部至少有一项真实内容时才显示，避免出现空白区块。
const showBasicInfoSection = computed(() => {
    return (
        props.productType !== "exchange" &&
        !isPresale.value &&
        (props.isWholesale || showMarketPriceInfo.value || showProductPriceInfo.value || hasRankDetail.value || showPromotionInfo.value)
    );
});

// 预售商品优惠信息
const presalePromotionList = ref<any[]>([]);
const getPresalePromotionList = async () => {
    if (!isPresale.value || !props.product?.productId) return;
    try {
        const result = await getPromotion({ products: [{ productId: props.product.productId }], from: "list" });
        presalePromotionList.value = result[props.product.productId]?.activityInfo || [];
    } catch (error) {
        console.error("获取预售优惠活动失败", error);
    }
};
if (isPresale.value) {
    getPresalePromotionList();
}

// 预售价格（定金预售显示定金，全款预售显示预售价）
const presalePriceValue = computed(() => {
    if (!isPresale.value) return props.product?.productPrice || "";
    if ((props.presaleDetail as any).saleMode === 2) return (props.presaleDetail as any).depositAmount || "";
    return (props.presaleDetail as any).presalePrice || props.product?.productPrice || "";
});

// 尾款支付时间显示
const finalPaymentTimeDisplay = computed(() => {
    if (!isPresale.value) return "";
    const info: any = props.presaleDetail;
    if (info.finalPaymentStartTimeText) return info.finalPaymentStartTimeText;
    if (info.finalPaymentType === 1) {
        return `${t("付定金后")}${info.finalPaymentRelativeDays}${t("天开始支付尾款，并需要在")}${info.finalPaymentDurationDays}${t("天")}${info.finalPaymentDurationHours}${t("小时内完成支付")}`;
    }
    return `${info.finalPaymentStartTime}-${info.finalPaymentEndTime}`;
});

// 全款预售发货时间显示
const deliveryTimeTextForFullPresale = computed(() => {
    if (!isPresale.value) return "";
    const info: any = props.presaleDetail;
    if (info.deliveryType === 1) {
        const days = info.deliveryRelativeDays ? `${info.deliveryRelativeDays}${t("天")}` : "";
        const hours = info.deliveryRelativeHours ? `${info.deliveryRelativeHours}${t("小时")}` : "";
        return `${t("付全款")}${days}${hours}${t("后")}`;
    }
    return `${info.deliveryDate ?? ""}${t("开始")}`;
});

// 门店相关
const currentShopName = ref(props.storeInfo.shopTitle);

watch(
    () => props.storeInfo.shopTitle,
    (newValue) => {
        currentShopName.value = newValue;
    }
);

const productRegionIds = useCookie<any[]>("productRegionIds");
if (productRegionIds.value) {
    regionIds.value = productRegionIds.value;
}

const onProductSkuChange = (item: any) => {
    if (item !== null) {
        skuId.value = item.skuId;
        // 预约商品先用规格库存；普通商品等可用性接口返回后再更新，避免详情规格库存为 0 时误显示缺货
        if (props.productType == "product" && isReservationProduct.value) {
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
    const requestId = ++latestLoadPriceRequestId;
    skuPriceLoading.value = true;
    try {
        const result = await getProductSkuDetail(props.id, {
            skuId: skuId.value ? skuId.value : null,
            extraAttrIds: attrIds.value,
            shopId: props.product?.shopId
        });
        if (requestId !== latestLoadPriceRequestId) return;
        productPrice.value = result.price;
        const skuMarketPrice = result?.sku?.marketPrice ?? result?.marketPrice;
        if (skuMarketPrice !== undefined && skuMarketPrice !== null && skuMarketPrice !== "") {
            props.product.marketPrice = String(skuMarketPrice);
        }
        const selectedSku = props.skuList.find((item) => Number(item.skuId) === Number(skuId.value));
        const selectedSkuStock = Number(selectedSku?.skuStock);
        const apiStock = Number(result.stock);
        // 可售库存以可用性接口的 stock 为准；预约商品使用规格库存
        if (isReservationProduct.value && Number.isFinite(selectedSkuStock)) {
            productStock.value = selectedSkuStock;
        } else if (Number.isFinite(apiStock)) {
            productStock.value = apiStock;
        } else {
            productStock.value = 0;
        }

        // 是否存在营销活动
        if (result.promotion.length > 0) {
            promotionManage(result.promotion);
        }

        if (isSeckill.value && maxProductNumber.value > 0) {
            productNumber.value = productNumber.value > maxProductNumber.value ? maxProductNumber.value : productNumber.value;
        } else if (Number(productStock.value) <= 0) {
            productNumber.value = 1;
        } else {
            productNumber.value = productNumber.value > productStock.value ? productStock.value : productNumber.value;
        }

        // 预售商品SKU切换时同步定金/尾款金额
        if (isPresale.value) {
            (props.product as any).productPrice = result.price || "0";
            (props.presaleDetail as any).depositAmount = result.depositAmount || "0";
            (props.presaleDetail as any).finalPaymentAmount = result.finalPaymentAmount || "0";
        }
    } catch (error) {
    } finally {
        if (requestId === latestLoadPriceRequestId) {
            skuPriceLoading.value = false;
        }
    }
};

if (props.skuList.length === 0) {
    loadPrice();
}

// 营销活动管理
const promotionManage = (promotion: SkuPromotion[]) => {
    // 排除秒杀 / 限时折扣 / 拼团 / 抽奖（优惠区只展示券、满减、满折、满赠）
    promotionList.value = promotion.filter((item: any) => ![1, 6, 7, 8].includes(item.type));
    console.log("promotionManage", promotionList.value);
    const seckillData = promotion.find((item) => item.type === 1);
    const discountData = promotion.find((item) => item.type === 6);

    // 判断限时折扣存在
    if (discountData) {
        const canUseDiscount = discountData.canJoin !== false;
        isDiscount.value = canUseDiscount ? 1 : 0;
        discountValue.value.discountEndTime = discountData.data.item.endTime!;
        discountValue.value.discountPrice = discountData.data.item.discountPrice!;
        discountValue.value.canJoin = canUseDiscount;
        discountValue.value.restrictTip = discountData.restrictTip || "";

        // 商详接口为展示活动仍会返回折扣价；无参与资格时还原为该会员的普通成交价
        if (!canUseDiscount && !seckillData) {
            const currentPrice = Number(productPrice.value);
            const discountAmount = Number(discountData.data?.item?.discountPrice ?? 0);
            if (Number.isFinite(currentPrice) && Number.isFinite(discountAmount)) {
                productPrice.value = (currentPrice + discountAmount).toFixed(2);
            }
        }
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
        seckillValue.value.seckillSales = seckillData.data.item.seckillSales!; // 秒杀销量
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

// b2b
const tabIndex = ref(0);
const isBatch = computed(() => {
    return tabIndex.value === 1 && props.product.productType !== 4;
});

// 是否展示内联购买按钮：新版 UI 下购买操作（含 B2B 批量采购）统一由悬浮购买栏承载，仅老版展示内联按钮
const showInlineBuy = computed(() => props.uiVersion !== "new");

// 悬浮购买栏绑定的批发加购/购买状态与事件，默认不加载、空操作
let wholesaleLoadingAdd: any = ref(false);
let wholesaleLoadingBuy: any = ref(false);
let onWholesaleAddToCart: any = () => {};
let onWholesaleBuyNow: any = () => {};


// 监听 product 变化，更新当前门店名称
watch(
    () => props.product?.shopId,
    (newShopId, oldShopId) => {
        // 如果 shopId 变化了，且不是通过选择门店触发的（避免重复设置）
        if (newShopId !== oldShopId && newShopId !== undefined && newShopId !== null && newShopId !== 0) {
            // 如果当前门店名称为空或者是旧的门店名称，则清空等待刷新
            // 因为路由变化后会触发 Product.vue 重新加载商品详情，商品详情中可能包含门店信息
            if (!currentShopName.value || props.product?.shopId !== newShopId) {
                currentShopName.value = "";
            }
        } else if (newShopId === undefined || newShopId === null || newShopId === 0) {
            currentShopName.value = "";
        }
    },
    { immediate: true }
);

defineExpose({ showEnquiry });
</script>
<style lang="less" scoped>
.product_info {
    // 二期会员标签：不可购买红字提示，置于数量选择下方，老版/新版共用。
    // 老版 .number-box 上 15px 下 25px，负 margin 抵消这 10px 差，使数量行上下间距一致
    .member-restrict-tip {
        display: flex;
        align-items: center;
        margin-top: -10px;
        margin-bottom: 16px;
        color: #f54e4e;
        font-size: 13px;

        .iconfont-pc {
            margin-right: 4px;
            font-size: 14px;
        }
    }

    &.new-ui {
        font-family: Inter, "SF Pro Text", Roboto, "Helvetica Neue", Helvetica, Tahoma, Arial, "PingFang SC", "Microsoft YaHei";

        // 新版 .number-box 上下均为 24px，无需抵消
        .member-restrict-tip {
            margin-top: 0;
        }

        .basic {
            box-sizing: border-box;
            display: flex;
            flex-direction: column;
            justify-content: center;
            gap: 8px;
            // max-width: 490px;
            min-height: 78px;
            margin: 12px 0;
            padding: 16px;
            border-radius: 12px;
            overflow: hidden;

            &.isActive {
                margin-top: 0 !important;
                border-radius: 0 0 12px 12px;
            }

            // 批发态（含阶梯报价表）：与标准版一致，去掉全部内边距
            &.basic-wholesale {
                margin-top: 0;
                margin-bottom: 0;
                padding: 0;
            }

            .item {
                padding: 0;
            }

            .item:not(.exchange) {
                align-items: baseline;
                flex-wrap: wrap;
                row-gap: 4px;
            }

            // 批发态：阶梯报价表与评价信息竖排换行展示；
            // 阶梯条占满整行宽度并允许内部溢出，使阶梯表能检测到超宽并显示左右滚动箭头（与跨境风格保持一致）
            .item-wholesale {
                flex-direction: column;
                align-items: stretch;
                row-gap: 8px;

                .price1-wholesale {
                    width: 100%;
                    min-width: 0;
                }

                // 评价信息左对齐，并提升层级避免被阶梯条左侧白色渐变遮罩盖住
                .rank-info {
                    align-self: flex-start;
                    justify-content: flex-start;
                    position: relative;
                    z-index: 2;
                }
            }

            .rank-info {
                align-items: baseline;
            }

            .tit {
                color: #999;
                min-width: 60px;
                font-size: 14px;
                font-weight: 500;
            }

            .item .price1 :deep(.price) {
                font-size: 28px;
                font-weight: 600;
                letter-spacing: -0.02em;
                line-height: 1;

                .num {
                    line-height: 1;
                }

                .util {
                    font-size: 14px;
                    line-height: 1;
                }
            }

            .tit2 {
                font-size: 14px;
                font-weight: 500;
            }

            .star {
                font-size: 14px;
                font-weight: 500;
            }

            .evaluate {
                font-size: 14px;
                font-weight: 500;
            }
        }

        .basic-info {
            box-sizing: border-box;
            display: flex;
            flex-direction: column;
            gap: 4px;
            margin: 24px 0 24px 0;

            .item {
                display: flex;
                align-items: center;
                padding: 0;
                .tit {
                    color: rgba(9, 9, 10, 0.92);
                    min-width: 50px;
                    font-size: 14px;
                    font-weight: 500;
                    line-height: 22px;
                }
                .num {
                    color: rgba(9, 9, 10, 0.92);
                    font-size: 14px;
                    font-weight: 500;
                    line-height: 22px;
                }
                .tips_i_box {
                    display: flex;
                    flex-wrap: wrap;
                    gap: 16px;
                    width: 95%;
                    span {
                        display: flex;
                        align-items: center;
                        line-height: 22px;
                        color: rgba(9, 9, 10, 0.92);
                        font-size: 14px;
                        font-weight: 500;
                    }
                }
            }
        }

        .promotion-tags {
            margin: 0 0 12px;

            :deep(.coupon-list-wrap) {
                row-gap: 8px;

                .promotion-box {
                    row-gap: 8px;

                    .promotion-box-item {
                        margin-right: 8px;
                    }

                    .promotion:not(:has(*)) {
                        display: none;
                    }

                    .promotion {
                        color: var(--tag-text, #cf0000);
                        border-radius: 8px;
                        height: 32px;
                        line-height: 32px;
                        padding: 0 12px;
                        font-size: 14px;
                        font-weight: 500;
                        overflow: hidden;
                        white-space: nowrap;
                        word-break: keep-all;

                        .ticket-type {
                            padding: 0 8px 0 0;
                            max-width: none;
                            position: relative;
                        }

                        .ticket-line {
                            right: 0;
                        }

                        .ticket-content {
                            padding: 0;
                            max-width: none;
                            overflow: hidden;
                            text-overflow: ellipsis;
                            white-space: nowrap;
                        }

                        .ticket-type + .ticket-content {
                            padding-left: 8px;
                        }
                    }
                }

                .lyecsquan-more {
                    font-size: 14px;
                }
            }
        }

        .actions {
            // 新版 UI 下即使存在 ProductB2bTab（.no-border）也保留顶部分隔线
            // &.no-borders {
            //     border-top: 1px solid #dddddd;
            // }
            .split-line {
                border-top: 1px solid #ebebeb;
                margin-bottom: 24px;
            }
            .number-box {
                display: flex;
                flex-direction: column;
                align-items: flex-start;
                gap: 12px;
                margin-bottom: 24px;

                .title {
                    color: #999;
                    height: auto;
                    line-height: 22px;
                    padding-right: 0;
                    font-size: 14px;
                    font-weight: 500;
                }

                .con {
                    display: flex;
                    align-items: center;
                    gap: 12px;
                }
            }
        }

        .active-box {
            margin-top: 12px !important;
            // max-width: 490px;
            border-radius: 12px 12px 0 0 !important;
        }

        .presalePrice {
            // max-width: 490px;
            margin-bottom: 24px;
            border-radius: 0 0 12px 12px !important;
        }

        .serviceAction {
            box-sizing: border-box;
            width: 100%;
            // max-width: 490px;
            margin-top: 4px;
            border-top: 1px solid #f0f0f0;
            overflow: hidden;

            :deep(.el-collapse) {
                border-top: none;
                border-bottom: 1px solid #f0f0f0;
            }

            :deep(.el-collapse-item__header) {
                height: auto;
                min-height: 30px;
                line-height: 1.4;
                padding: 12px 0;
                border-bottom: 1px solid #f0f0f0;
                color: rgba(9, 9, 10, 0.92);
                font-size: 15px;
                font-weight: 600;
                white-space: normal;
                word-break: break-word;
                overflow-wrap: anywhere;

                &.is-active {
                    border-bottom-color: transparent;
                }
            }

            // 关掉高度过渡，避免收起时文字/边框叠成深色线再变回灰色
            :deep(.collapse-transition) {
                transition: none !important;
            }

            :deep(.el-collapse-item__wrap) {
                border-bottom: none !important;
                overflow: hidden;
                background-color: transparent;
                opacity: 0;
            }

            :deep(.el-collapse-item.is-active .el-collapse-item__wrap) {
                opacity: 1;
            }

            :deep(.el-collapse-item.is-active .el-collapse-item__wrap)::after {
                content: "";
                display: block;
                border-bottom: 1px solid #f0f0f0;
            }

            :deep(.el-collapse-item__content) {
                padding: 0 0 16px;
                color: #999;
                font-size: 14px;
                font-weight: 500;
                line-height: 22px;
                overflow: hidden;
            }

            .service-desc {
                min-width: 0;

                span {
                    display: block;
                    min-width: 0;
                    color: #999;
                    font-size: 14px;
                    font-weight: 500;
                    line-height: 22px;
                    white-space: normal;
                    word-break: break-word;
                    overflow-wrap: anywhere;
                }
            }

            .collapse-icon {
                flex-shrink: 0;
                display: flex;
                align-items: center;

                svg {
                    transition: transform 0.3s ease;
                }

                &.active svg {
                    transform: rotate(180deg);
                }
            }
        }
    }

    .bg {
        background: url("/assets/images/product/price-bg.png") repeat-x scroll 0 bottom #f3f3f3;
        padding-bottom: 12px;
        padding-top: 10px;
    }

    .active-box {
        background: url("/assets/images/product/pro-time-bg.png") no-repeat;
        background-size: 100% 100%;
        padding: 13px 0;
        border-radius: 6px 6px 0 0;
        font-size: 16px;
        .active-ico {
            color: #fff;
            margin-left: 10px;
            font-size: 16px;
            gap: 8px;
            &.align-end {
                align-items: flex-end;
            }
            i {
                margin-right: 5px;
                font-size: 18px;
            }
            :deep(.el-icon) {
                display: flex;
                align-items: center;
                font-size: 16px;
                color: #fff;
            }
            .text {
                position: relative;
                margin-right: 26px;
                display: flex;
                align-items: center;
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
                        align-items: baseline;
                        color: #fff;
                        font-size: 22px;

                        .util {
                            color: #fff;
                            font-size: 12px;
                            line-height: 22px;
                        }
                        .num {
                            color: #fff;
                            line-height: 18px;
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

    &:not(.new-ui) {
        .basic-info {
            .item {
                display: flex;
                align-items: center;
                padding: 5px 10px;

                .tit {
                    color: #999;
                    min-width: 60px;
                }

                .num {
                    color: #999;
                }

                .tips_i_box {
                    display: flex;
                    flex-wrap: wrap;
                    width: 95%;

                    span {
                        display: flex;
                        align-items: center;
                        margin-left: 15px;
                        color: #999;
                        margin-bottom: 10px;
                        &:nth-child(1) {
                            margin-left: 0;
                        }
                    }
                }
            }
        }
    }

    .basic {
        .item {
            display: flex;
            align-items: center;
            padding: 5px 10px;

            .tit {
                color: #999;
                min-width: 60px;
            }

            .tit2 {
                color: #999;
            }

            .market_price {
                color: #888;
                text-decoration: line-through;

                :deep(.price) {
                    align-items: baseline;
                    vertical-align: baseline;
                }
            }

            .price1 {
                :deep(.price) {
                    align-items: baseline;
                    vertical-align: baseline;
                    color: var(--general);
                    font-size: 26px;
                    margin-right: 20px;

                    .util {
                        font-size: 12px;
                        line-height: 26px;
                    }
                    .num {
                        line-height: 18px;
                    }
                }
            }

            // 批发态：阶梯报价表与评价信息竖排换行展示；
            // 阶梯条占满整行宽度并允许内部溢出，使阶梯表能检测到超宽并显示左右滚动箭头
            &.item-wholesale {
                flex-direction: column;
                align-items: stretch;
                row-gap: 8px;

                .price1-wholesale {
                    width: 100%;
                    min-width: 0;
                }

                // 评价信息换行左对齐，并提升层级避免被阶梯条左侧白色渐变遮罩盖住
                .rank-info {
                    align-self: flex-start;
                    justify-content: flex-start;
                    position: relative;
                    z-index: 2;
                }
            }

            .rank-info {
                display: flex;
                align-items: center;
            }

            .evaluate {
                margin-left: 10px;
                color: #999;
            }

            .num {
                color: #999;
            }

            .tips_i_box {
                display: flex;
                flex-wrap: wrap;
                width: 95%;

                span {
                    display: flex;
                    align-items: center;
                    margin-right: 15px;
                    color: #999;
                    margin-bottom: 10px;
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

    // 预售商品相关样式
    .presale-sales {
        margin-bottom: 15px;
        font-size: 14px;
        color: #999;
        display: flex;
        align-items: center;
        gap: 8px;
        margin-left: 15px;
        span {
            color: #999;
        }
    }
    .presale-sales-divider {
        height: 1px;
        background-color: #eee;
        margin-bottom: 15px;
    }
    .presalePrice {
        background: url("/assets/images/product/price-bg.png") repeat-x scroll 0 bottom #f3f3f3;
        padding: 15px;
        box-sizing: border-box;
        margin-bottom: 25px;
        border-radius: 0 0 6px 6px;

        .presale-info-content {
            display: flex;
            flex-direction: column;
            gap: 12px;
            font-size: 14px;
            color: #666;

            .presalePriceLine {
                display: flex;
                align-items: baseline;
                gap: 8px;

                .price-label {
                    display: inline-block;
                    min-width: 70px;
                    color: #666;
                }

                .priceBox {
                    height: 24px;
                }

                &.deposit-line {
                    :deep(.price) {
                        color: #999;
                        align-items: baseline !important;
                    }
                }

                &.presale-price-line {
                    :deep(.price) {
                        color: var(--general);
                        align-items: baseline !important;
                    }
                }
            }
        }
    }

    .presale-final-payment {
        margin-bottom: 15px;
        font-size: 14px;
        margin-left: 15px;

        .final-payment-content {
            display: flex;
            align-items: baseline;
            gap: 8px;
            flex-wrap: wrap;

            .final-payment-label {
                color: #999;
            }

            .final-payment-amount-box {
                display: flex;
                flex-direction: column;
                gap: 4px;
                .final-payment-amount {
                    height: 24px;
                }
                :deep(.price) {
                    color: var(--general);
                }
                .final-payment-time {
                    font-size: 12px;
                }
            }
        }
    }

    .presale-process {
        margin-bottom: 25px;
        font-size: 14px;
        display: flex;
        align-items: flex-end;
        gap: 8px;
        margin-left: 15px;

        .process-label {
            color: #999;
            display: flex;
            align-items: flex-end;
        }

        .process-text {
            color: #333;
            display: flex;
            align-items: flex-end;
        }

        .presale-info-icon {
            color: #999;
            font-size: 14px;
            cursor: pointer;
            display: flex;
            align-items: flex-end;
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

    .actions {
        // border-top: 1px solid #dddddd;
        overflow: hidden;
        // padding: 15px 1px 0 10px;
        &.no-border {
            border-top: none;
        }

        .number-box {
            display: flex;
            margin-bottom: 25px;
            margin-top: 15px;

            .title {
                color: #999;
                height: 33px;
                line-height: 32px;
                padding-right: 10px;
            }

            .con {
                display: flex;
                align-items: center;
            }
        }

        .add_cart {
            display: flex;
            flex-wrap: wrap;
            margin-left: 45px;
            padding-bottom: 25px;

            &.presale-cart {
                margin-left: 0;
            }

            .buy-btn {
                height: 40px;
                padding: 0 20px;
                border-radius: 2px;
                &:not(:disabled):not(.is-disabled):hover {
                    opacity: 0.9;
                }
                margin-right: 10px;

                &.presaleBuy {
                    padding: 20px 45px;
                    font-weight: 500;
                    font-size: 16px;
                }
            }

            // 禁用态对齐 uni 端：沿用可用态皮肤，只做整体淡化。
            // Element Plus 的 .is-disabled 会改用 --el-button-disabled-* 另画一套灰色皮肤，把这组变量指回可用态变量即可抵消。
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
                height: 40px;
                padding: 0 20px;
            }
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

    // 仅调整标准版批发态的模块外沿节奏，经典版保留原有布局。
    // 阶梯价格内部的价格与起批量间距仍由 WholesalePriceTable 保持 10px。
    &.standard-wholesale-spacing {
        --standard-wholesale-section-gap: 24px;

        padding-top: var(--standard-wholesale-section-gap);

        .basic-info {
            margin-top: var(--standard-wholesale-section-gap);
            margin-bottom: 0;
        }

        .basic {
            .item.item-wholesale {
                row-gap: var(--standard-wholesale-section-gap);
            }
        }

        :deep(.product-store-card .store-item-wrapper) {
            margin-top: 24px;
            margin-bottom: 24px;
        }

        :deep(.wholesale-sku-selector > .dim-group:first-child) {
            padding-top: 0;
        }

        // 商品金额上下留白保持一致；存在服务区时由合计行自身承担间距，避免再叠加 serviceAction 的 4px。
        :deep(.wholesale-sku-selector.new-ui .picked-summary) {
            padding-top: 16px;
            padding-bottom: 16px;
        }

        .serviceAction {
            margin-top: 0;
        }

        // 标准版批发价由内容自然撑高，避免 78px 最小高度给短价格形态制造隐形留白。
        .basic.basic-wholesale {
            min-height: 0;
        }

        // 单档/区间价取消等高占位；多档阶梯取消内部上下 8px，三种价格形态共用同一外部节奏。
        :deep(.wholesale-price-range) {
            min-height: 0;
            align-items: flex-start;
        }

        :deep(.wholesale-ladder-bar .ladder-scroll) {
            padding-top: 0;
            padding-bottom: 0;
        }

        &.wholesale-no-store-spacing {
            .basic-info {
                margin-bottom: var(--standard-wholesale-section-gap);
            }
        }

        &.has-product-brief {
            padding-top: 12px;
        }
    }
}
</style>
