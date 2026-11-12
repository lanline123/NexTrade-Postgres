<template>
    <div class="order-check-page">
        <CommonPageHeader></CommonPageHeader>
        <CommonHeader title="订单结算页"></CommonHeader>

        <CommonPageContainer>
            <div class="flex_csb">
                <div data-form-rev="ajIyODJ5ZWk" class="orderConfirmCon">
                    <div class="orderConfirmConTop">
                            <Address
                                ref="addressRef"
                                v-model="showMoreAddress"
                                :active-address-id="formState.addressId"
                                :addressList="addressList"
                                @addrssRefresh="addressRefresh"
                                @okCallback="onChangeAddress"
                            ></Address>
                    </div>
                    <div class="orderConfirmConTop">
                        <template v-if="addressList && addressList.length > 0">
                            <PayType :pay-type-id="formState.payTypeId" :payment-type-list="paymentTypeList" @changePayType="onChangePayType"></PayType>
                        </template>
                    </div>
                    <template v-if="cartList && cartList.length > 0">
                        <div class="orderConfirmConTop" v-for="(store, index) in cartList" :key="store.shopId">
                            <div class="form_item">
                                <div class="product_detail">
                                    <div class="shop_name">{{ store.shopId == 0 ? $t("自营") : `${$t("商家")}：${store.shopTitle}` }}</div>
                                    <div class="delivery" v-if="!isStore()">
                                        <div class="tit">{{ $t("配送") }}</div>
                                        <template v-if="store.noShipping === 1">
                                            <div class="delivery_tags flex_cc">
                                                <div class="pay_tab flex align-center active">
                                                    <span>{{ $t("无需配送") }}</span>
                                                </div>
                                            </div>
                                        </template>
                                        <template v-else-if="shippingTypeData[store.shopId] && shippingTypeData[store.shopId].length > 0">
                                            <div class="delivery_tags flex_cc">
                                                <template v-for="(item, key) in shippingTypeData[store.shopId]" :key="item.shippingTypeId || key">
                                                    <div
                                                        class="pay_tab flex align-center"
                                                        :class="{
                                                            active:
                                                                formState.shippingType?.[store.shopId] &&
                                                                item.shippingTypeId === formState.shippingType[store.shopId].typeId
                                                        }"
                                                        @click="onChangeShippingType(store.shopId, item)"
                                                    >
                                                        <span>{{ item.shippingTypeName }} </span>
                                                    </div>
                                                </template>
                                            </div>
                                        </template>
                                        <template v-else>
                                            <div class="no_delivery">
                                                <span class="errInfo red" style="display: inline; margin: 0">{{ $t("该收货地址不支持配送") }}</span>
                                            </div>
                                        </template>
                                    </div>
                                    <div class="detail">
                                        <div class="list flex align-start" v-for="product in store.carts">
                                            <div class="flex align-start">
                                                <div class="pic">
                                                    <NuxtLink
                                                        target="_blank"
                                                        :to="urlFormat({ path: 'product', seoLink: product.seoLink, id: product.productId, sn: product.productSn })"
                                                    >
                                                        <img :src="productSkuImage(product)" :alt="product.productName || ''" />
                                                    </NuxtLink>
                                                </div>
                                                <div class="describe flex flex-column">
                                                    <div class="name ellipsis" :title="product.productName">
                                                        <NuxtLink
                                                            target="_blank"
                                                            :to="urlFormat({ path: 'product', seoLink: product.seoLink, id: product.productId, sn: product.productSn })"
                                                        >
                                                            <!-- <span
                                                                class="seckill-item-discount"
                                                                v-if="!product.productPrice || product.productPrice != product.price"
                                                            >
                                                                {{ $t("优惠") }}
                                                                {{ getDiscount(Number(product.price), Number(product.productPrice)) }}%
                                                            </span> -->

                                                            <span class="discount-txt" v-if="!product.productPrice || product.productPrice != product.price">
                                                                {{ $t("优惠{0}%", [calcDiscountPercent(Number(product.price), Number(product.productPrice))]) }}
                                                            </span>
                                                            {{ product.productName }}
                                                        </NuxtLink>
                                                    </div>
                                                    <div class="mt10 sku-name-box">
                                                        <div v-for="(sku, skuIndex) in product.skuData" class="sku-name-item">
                                                            <span class="sku-name">{{ sku.name }}：</span>{{ sku.value }}
                                                        </div>
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
                                                                        <FormatPrice v-model="attr.attrPrice" :showText="false"></FormatPrice>
                                                                    </div>
                                                                </div>
                                                            </template>
                                                        </div>
                                                    </template>
                                                    <template v-if="product.activityInfo && activityList(product.activityInfo).length > 0">
                                                        <div class="activity-box">
                                                            <template
                                                                v-for="activityItem in activityList(product.activityInfo)"
                                                                :key="activityItem.promotionId"
                                                            >
                                                                <div class="promotion">
                                                                    <div class="ticket-content">
                                                                        {{ activityItem.type === 1 ? $t("秒杀价") : $t("限时折扣") }}
                                                                    </div>
                                                                </div>
                                                            </template>
                                                        </div>
                                                    </template>
                                                </div>
                                            </div>
                                            <div class="money">
                                                <FormatPrice
                                                    v-model="product.price"
                                                    :fontStyle="{ fontSize: '16px', color: '#1a1a1a', lineHeight: '16px' }"
                                                    :decimalsStyle="{ fontSize: '14px', color: '#1a1a1a' }"
                                                    :currencyStyle="{ fontSize: '14px', color: '#1a1a1a' }"
                                                ></FormatPrice>
                                            </div>
                                            <div class="quantity">x{{ product.quantity }}</div>
                                        </div>
                                        <template v-if="store.gift && store.gift.length > 0">
                                            <div class="gift-box">
                                                <div class="gift-title">{{ $t("赠品") }}</div>
                                                <div class="gift-content">
                                                    <template v-for="subItem in store.gift" :key="subItem.giftId">
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
                                                                    <el-image
                                                                        class="gift-img"
                                                                        lazy
                                                                        :src="productSkuImage(subItem)"
                                                                        :alt="subItem.productName || ''"
                                                                    >
                                                                    </el-image>
                                                                </div>
                                                                <div class="gift-item-right">
                                                                    <div class="gift-item-name">{{ subItem.productName }}</div>
                                                                    <template v-if="subItem.skuData && subItem.skuData.length > 0">
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
                                </div>
                            </div>
                        </div>
                    </template>

                    <!-- 预售商品步骤条 -->
                    <template v-if="productPresale">
                        <div class="orderConfirmConTop presale-steps">
                            <PresaleSteps v-if="!isStore()" :productPresale="productPresale" :presaleStatus="0" :total="total" />
                        </div>
                    </template>

                    <div class="orderConfirmConTop">
                        <div class="form_item">
                            <div class="add_remark">
                                <div class="a_rmk_tit">{{ $t("订单备注") }}</div>
                                <el-input
                                    size="large"
                                    v-model="formState.buyerNote"
                                    :placeholder="$t('提示：请勿填写有关支付、收货、发票方面的信息')"
                                    style="flex: 1"
                                    show-word-limit
                                    :maxlength="50"
                                />
                            </div>
                        </div>
                    </div>
                    <template v-if="!useGiftCard">
                        <div class="orderConfirmConTop" v-if="commonStore.useCoupon != 0">
                            <CouponInfo
                                v-model:useCoupon="useCoupon"
                                v-model:couponList="couponList"
                                v-model:selectedIds="formState.useCouponIds"
                                v-model:selectedUserCouponIds="formState.selectUserCouponIds"
                                @change="onCouponChange"
                                v-model:couponLoading="couponLoading"
                            ></CouponInfo>
                        </div>
                        <div class="orderConfirmConTop" v-if="commonStore.usePoints != 0 && flowType === 1">
                            <div class="from">
                                <div class="check-boxPoint flex">
                                    <span class="tit">{{ $t("使用积分") }}</span>
                                    <div style="margin-left: 10px; font-size: 14px">
                                        {{ $t("当前账户共") }} <span class="font-color"> {{ userStore.userInfo.points }} </span> {{ $t("积分") }}
                                    </div>
                                </div>
                            </div>

                            <div class="from_list">
                                <div class="integral flex">
                                    <div class="tit">{{ $t("使用积分") }}</div>
                                    <div class="inp_box">
                                        <div class="inp">
                                            <el-input style="width: 100%" size="large" :max="availablePoints" v-model="inputPoint" @change="usePointChange" />
                                        </div>
                                        <div class="tips" v-if="total.pointsAmount > 0">
                                            {{ $t("共抵扣") }}
                                            <FormatPrice class="red" v-model="total.pointsAmount" :showText="false" />
                                        </div>
                                        <div class="tips">
                                            {{ $t("该订单最多可用{0}{1}", [availablePoints, $t("积分")]) }}
                                            <a href="/article/issue/info?articleId=7" target="_blank"
                                                ><span class="font-color">【{{ $t("如何获得积分") }}?】</span></a
                                            >
                                        </div>
                                    </div>
                                </div>
                            </div>
                        </div>
                    </template>
                    <div class="orderConfirmConTop">
                        <template v-if="titleText">
                            <div class="form_item">
                                <div class="top_row">
                                    <div class="tit">{{ $t("余额") }}</div>
                                </div>
                                <client-only>
                                    <div class="from" v-if="commonStore.useSurplus != 0">
                                        <div class="check-box flex align-center">
                                            <el-checkbox :model-value="formState.useBalance > 0" @change="onUseBalance" size="large">
                                                <span class="tit">{{ $t("使用余额") }}</span></el-checkbox
                                            >
                                            <div style="margin-left: 10px">
                                                {{ $t("当前账户余额") }} <span class="font-color">{{ userStore.userInfo.balance }} </span>
                                            </div>
                                        </div>
                                    </div>
                                    <!-- <div
                                        v-if="formState.useBalance > 0"
                                        style="color: #333; display: flex; align-items: baseline; margin-left: 27px; font-size: 14px"
                                    >
                                        <span style="margin-right: 5px">{{ $t("支付") }}</span>
                                        <FormatPrice :font-style="{ color: 'var(--general)', fontSize: '14px' }" v-model="total.balance" :showText="false" />
                                    </div> -->
                                </client-only>
                            </div>
                        </template>
                    </div>
                    <div class="orderConfirmConTop" v-if="commonStore.canInvoice == 1 && !useGiftCard">
                        <div class="form_item">
                            <div class="top_row flex align-end">
                                <div class="tit">{{ $t("发票信息") }}</div>
                                <div class="tips">{{ $t("开企业抬头发票须填写纳税人识别号，以免影响报销") }}</div>
                            </div>
                            <div class="from">
                                <div class="flex align-center invoice-box">
                                    <div class="title flex align-center">
                                        <span>{{
                                            !formState.invoiceData.invoiceType
                                                ? $t("不开发票")
                                                : formState.invoiceData.invoiceType == 1
                                                  ? $t("电子普通发票")
                                                  : $t("增值税专用发票")
                                        }}</span>

                                        <template v-if="formState.invoiceData.invoiceType == 1">
                                            <el-tooltip
                                                :content="$t('电子普通发票与纸质发票具有同等法律效力，可支持报销入账、商品售后凭证。')"
                                                placement="bottom-start"
                                                effect="light"
                                            >
                                                <i class="iconfont-pc icon-tishi"></i>
                                            </el-tooltip>
                                        </template>
                                    </div>
                                    <div v-if="formState.invoiceData.invoiceType" class="type">
                                        <span v-if="formState.invoiceData.invoiceType == 1">{{
                                            formState.invoiceData.titleType == 1 ? $t("个人") : $t("企业")
                                        }}</span>
                                        <span v-if="formState.invoiceData.invoiceType == 2">{{ $t("增值税专用发票") }}</span>
                                    </div>
                                    <div class="type">
                                        <span>{{ formState.invoiceData.companyName ?? "" }}</span>
                                    </div>
                                    <InvoiceDialog
                                        :params="{
                                            invoiceData: formState.invoiceData,
                                            amount: total?.unpaidAmount,
                                            type: 'check',
                                            address: addressList.find((item: any) => formState.addressId == item.addressId)
                                        }"
                                        @callback="getInvoiceData"
                                        @changeType="changeType"
                                    >
                                        <div class="btn hand-pointer">{{ $t("修改") }}</div>
                                    </InvoiceDialog>
                                </div>
                            </div>
                        </div>
                    </div>
                    <div v-if="isB2B() && creditPaymentInfo && creditPaymentInfo.canUseCredit" class="orderConfirmConTop">
                        <div class="form_item">
                            <div class="top_row">
                                <div class="tit">{{ $t("账期") }}</div>
                            </div>
                            <client-only>
                                <div class="from">
                                    <div class="check-box flex align-center" :class="{ 'credit-disabled': creditPaymentInfoDisabled }">
                                        <el-checkbox
                                            :disabled="creditPaymentInfoDisabled"
                                            :model-value="formState.useCredit === 1"
                                            @change="handleCredit"
                                            size="large"
                                        >
                                            <span class="tit">{{ $t("账期支付") }}</span></el-checkbox
                                        >
                                        <div v-if="creditPaymentInfoDisabledText" style="margin-left: 10px">
                                            <span class="font-color">{{ creditPaymentInfoDisabledText }} </span>
                                        </div>
                                        <div v-else style="margin-left: 10px">
                                            {{ $t("当前可用额度") }} <span class="font-color">{{ creditPaymentInfo.availableAmount }} </span>
                                        </div>
                                    </div>
                                </div>
                            </client-only>
                        </div>
                    </div>
                </div>
                <div class="money_info">
                    <div class="money_title">{{ $t("订单详情") }}</div>
                    <ClientOnly>
                        <div class="total_list">
                            <div class="discount">
                                <div class="flex justify-between">
                                    <div class="total_text">{{ $t("商品总价") }}：</div>
                                    <FormatPrice
                                        class="total_text"
                                        v-model="total.productAmount"
                                        :showText="false"
                                        :fontStyle="{ fontWeight: 'bold', fontSize: '14px' }"
                                    />
                                </div>
                                <div class="flex justify-between" v-if="total.serviceFee && total.serviceFee > 0">
                                    <div class="total_text">{{ $t("附加费用") }}：</div>
                                    <FormatPrice
                                        class="total_text"
                                        v-model="total.serviceFee"
                                        :showText="false"
                                        :fontStyle="{ fontWeight: 'bold', fontSize: '14px' }"
                                    />
                                </div>
                                <div class="flex justify-between" v-if="total.balance > 0">
                                    <div class="total_text">{{ $t("使用余额") }}：</div>
                                    <div style="color: var(--tag-text)">
                                        -
                                        <FormatPrice
                                            class="total_text"
                                            v-model="total.balance"
                                            :showText="false"
                                            :fontStyle="{ fontWeight: 'bold', fontSize: '14px' }"
                                        />
                                    </div>
                                </div>
                                <div class="flex justify-between" v-if="(total as any).giftCardAmount && (total as any).giftCardAmount > 0">
                                    <div class="total_text">{{ $t("使用礼品卡") }}：</div>
                                    <div style="color: var(--tag-text)">
                                        -
                                        <FormatPrice
                                            class="total_text"
                                            v-model="(total as any).giftCardAmount"
                                            :showText="false"
                                            :fontStyle="{ fontWeight: 'bold', fontSize: '14px' }"
                                        />
                                    </div>
                                </div>
                                <div class="flex justify-between" v-if="total.pointsAmount > 0 && flowType != 3">
                                    <div class="total_text">{{ $t("积分减免") }}：</div>
                                    <div style="color: var(--tag-text)">
                                        -
                                        <FormatPrice
                                            class="total_text"
                                            v-model="total.pointsAmount"
                                            :showText="false"
                                            :fontStyle="{ fontWeight: 'bold', fontSize: '14px' }"
                                        />
                                    </div>
                                </div>
                                <div class="discount-content">
                                    <div class="flex justify-between discount-item-box" v-if="total?.discounts > 0">
                                        <div class="item-label">{{ $t("共优惠") }}</div>
                                        <div class="item-value discounts-value" style="color: var(--general)">
                                            -
                                            <format-price v-model="total.discounts" :fontStyle="{ fontWeight: 'bold', fontSize: '14px' }"></format-price>
                                        </div>
                                    </div>
                                    <div :class="'discount-item-box ' + (needShipping ? 'border-bottom' : '')" v-if="total?.productAmount > 0">
                                        <div class="total_text">{{ $t("商品总额") }}</div>
                                        <div class="item-value">
                                            <format-price v-model="total.productAmount" :fontStyle="{ fontWeight: 'bold', fontSize: '14px' }"></format-price>
                                        </div>
                                    </div>

                                    <div class="flex justify-between discount-item-box" v-if="total?.discountCouponAmount > 0">
                                        <div class="item-label">{{ $t("优惠券") }}</div>
                                        <div class="item-value discounts-value" style="color: var(--general)">
                                            -
                                            <format-price
                                                v-model="total.discountCouponAmount"
                                                :fontStyle="{ fontWeight: 'bold', fontSize: '14px' }"
                                            ></format-price>
                                        </div>
                                    </div>
                                    <div class="flex justify-between discount-item-box" v-if="total.discountDiscountAmount > 0">
                                        <div class="item-label">{{ $t("其他优惠") }}</div>
                                        <div class="item-value discounts-value">
                                            -
                                            <format-price
                                                v-model="total.discountDiscountAmount"
                                                :fontStyle="{ fontWeight: 'bold', fontSize: '14px' }"
                                            ></format-price>
                                        </div>
                                    </div>
                                    <div class="item" v-if="needShipping">
                                        <div class="flex">
                                            <div class="item-label">
                                                <template v-if="!isStore()">{{ $t("快递") }}：</template>
                                            </div>
                                            <div class="tip-item" v-if="!isStore()">
                                                <SvgIcon name="tips" class="freight-icon" />
                                                <div class="ui-tips">
                                                    <span class="ui-tips-arrow"></span>
                                                    <div class="ui-tips-main">
                                                        <div class="tit">{{ $t("运费明细") }}</div>
                                                        <div class="goods_list">
                                                            <div class="goods_col" v-for="(store, index) in cartList">
                                                                <div class="shop_name">{{ store.shopId == 0 ? $t("自营") : store.shopTitle }}</div>
                                                                <div class="list">
                                                                    <div class="txt">
                                                                        {{ $t("快递") }}：
                                                                        <FormatPrice
                                                                            v-if="Number(total.storeShippingFee[store.shopId]) > 0"
                                                                            :showText="false"
                                                                            v-model="total.storeShippingFee[store.shopId]"
                                                                            :fontStyle="{ fontWeight: 'bold', fontSize: '14px', color: '#1a1a1a' }"
                                                                        />
                                                                        <span v-else-if="!checkLoading">{{ $t("包邮") }}</span>
                                                                    </div>
                                                                    <div class="item" style="padding: 10px 0">
                                                                        <swiper
                                                                            v-model:swiperOption="swiperOption"
                                                                            :itemList="store.carts"
                                                                            style="width: 88%; padding: 0 20px"
                                                                        >
                                                                            <template v-slot:default="{ item }">
                                                                                <div class="img_box" style="width: 65px">
                                                                                    <img :src="productSkuImage(item)" alt="" />
                                                                                </div>
                                                                            </template>
                                                                        </swiper>
                                                                    </div>
                                                                </div>
                                                            </div>
                                                        </div>
                                                    </div>
                                                </div>
                                            </div>
                                        </div>
                                        <FormatPrice v-if="Number(total.shippingFee) > 0" class="total_text" :showText="false" v-model="total.shippingFee" />
                                        <template v-else-if="!checkLoading">
                                            <span v-if="!isStore()" class="free-text">{{ $t("包邮") }}</span>
                                        </template>
                                        <!-- 更新中占位符，避免 needShipping 立刻为 true 但 shippingFee 未到时闪一下「包邮」 -->
                                        <span v-else class="free-text" style="visibility: hidden">-</span>
                                    </div>
                                    <div class="discount-item-box" v-if="showTaxAmount">
                                        <div class="item-label tax-item-label">
                                            <span>{{ $t("税费") }}：</span>
                                            <el-tooltip placement="bottom-start" effect="light">
                                                <template #content>
                                                    <div class="tax-tip-content">
                                                        <div>{{ $t("税费说明") }}</div>
                                                        <div>{{ $t("税费按所选地址所在国家/地区政策计算。") }}</div>
                                                        <div>{{ $t("选择地址后，系统自动计算税费，结算页显示税费金额。") }}</div>
                                                        <div>{{ $t("税费会随订单一起支付，由商城统一代缴。") }}</div>
                                                    </div>
                                                </template>
                                                <SvgIcon name="tips" width="12" height="12" />
                                            </el-tooltip>
                                        </div>
                                        <div class="item-value">
                                            <format-price v-model="total.taxRateAmount" :fontStyle="{ fontWeight: 'bold', fontSize: '14px' }"></format-price>
                                        </div>
                                    </div>
                                    <div class="discount-item-box" v-if="total.discountAfter && total.discountAfter > 0">
                                        <div class="item-label">{{ $t("合计") }}</div>
                                        <div class="item-value">
                                            <format-price v-model="total.discountAfter" :fontStyle="{ fontWeight: 'bold', fontSize: '14px' }"></format-price>
                                        </div>
                                    </div>
                                </div>
                            </div>
                        </div>
                    </ClientOnly>
                    <div v-if="total.orderSendPoint && total.orderSendPoint > 0" class="complete">
                        *{{ $t("该订单完成后，您将预计获得{0}{1}", [total.orderSendPoint, commonStore.integralName]) }}
                    </div>
                    <div class="total_fee flex justify-between">
                        <div class="flex total_fee-box">
                            <div class="unpaidamount-label">{{ $t("仍需支付") }}：</div>
                            <template v-if="flowType == 3">
                                <div class="points-box">
                                    <div class="points-value">
                                        {{ total?.exchangePoints }}
                                        <div class="points-text">{{ $t("积分") }}</div>
                                    </div>
                                    <div class="symbol">+</div>
                                </div>
                            </template>
                        </div>
                        <FormatPrice class="price" v-model="payAmount" :showText="false" :fontStyle="{ fontWeight: 'bold', fontSize: '14px' }"></FormatPrice>
                    </div>
                    <div class="submit">
                        <!-- <el-button
                            type="primary"
                            class="btn buy-btn"
                            :disabled="submitDisabled"
                            :loading="submitLoading"
                            @click="!submitLoading && onSubmit()"
                            >{{ $t("立即下单") }}</el-button
                        > -->
                        <TigButton
                            theme
                            type="primary"
                            class="btn buy-btn"
                            :disabled="submitDisabled"
                            :loading="submitLoading"
                            @click="!submitLoading && onSubmit()"
                            >{{ isDepositPresale ? $t("支付定金") : $t("立即下单") }}</TigButton
                        >
                    </div>
                </div>
            </div>
        </CommonPageContainer>

        <ModalRegion v-if="editAddressShow" v-model="editAddressShow" :addressId="editAddressId"></ModalRegion>
        <CommonPageFooter></CommonPageFooter>
    </div>
</template>
<script lang="ts" setup>
/**
 * @description 订单确认页面
 * - 展示购物车选中商品，确认收货地址、配送方式、支付方式
 * - 支持使用优惠券、积分、余额抵扣
 * - 支持开具发票设置
 * - 提交订单后跳转到支付页面
 */
import { reactive, ref, computed, shallowRef } from "vue";
import CouponInfo from "./src/CouponInfo.vue";
import PresaleSteps from "@/pages/order/src/presaleSteps.vue";
import { ModalRegion } from "@/components/region";
import { getOrderCheck, updateOrderCheck, updateCouponData, orderSubmit, getCheckInvoice, getPaymentType, getShippingType } from "@/api/order/check";
import type { PaymentTypeItem, ShippingTypeItem, CheckCartList, CheckTotal, CheckCouponList, CreditPaymentInfo } from "~/types/order/check.d";
import { urlFormat, productSkuImage } from "@/utils/format";
import { message } from "ant-design-vue";
import { ElLoading } from "element-plus";
import InvoiceDialog from "~/components/invoice/InvoiceDialog.vue";
import Address from "@/pages/order/src/Address.vue";
import PayType from "@/pages/order/src/PayType.vue";
import { useCommonStore } from "~/store/common";
import { useUserStore } from "~/store/user";
import { isOverseas, isB2B, isStore } from "~/utils/util";
import { getAddressList, setSelectedAddress } from "@/api/user/address";
import type { AddressFilterState } from "~/types/user/address.d";
import InputNumber from "~/pages/cart/src/InputNumber.vue";
import { calcDiscountPercent } from "~/utils/product";
definePageMeta({
    middleware: "auth"
});

const { t } = useI18n();

const commonStore = useCommonStore();
const userStore = useUserStore();
const route = useRoute();

const flowType = computed(() => {
    return route.query.flowType ? Number(route.query.flowType) : 1;
});

// flowType 12为礼品卡兑换订单
const useGiftCard = ref(flowType.value === 12);


// 当前结算表单数据
interface IformState {
    addressId: number;
    shippingType: {
        [key: string]: {
            typeId: number;
            shopId: number;
            typeName: string;
        };
    };
    payTypeId: number;
    usePoint: number;
    useBalance: number;
    /** 是否使用账期支付：0-不使用，1-使用（与余额互斥） */
    useCredit: 0 | 1;
    useCouponIds: number[];
    selectUserCouponIds: number[];
    buyerNote: string;
    invoiceData: any;
    useDefaultCouponIds?: number;
}

const formState = reactive<IformState>({
    addressId: 0,
    shippingType: {},
    payTypeId: 0,
    usePoint: 0,
    useBalance: 0,
    useCredit: 0,
    useCouponIds: [],
    selectUserCouponIds: [],
    buyerNote: "",
    invoiceData: {},
});
const addressList = ref<AddressFilterState[]>([]);
const addressRef = useTemplateRef<any>("addressRef");
/**
 * @description 获取用户收货地址列表，若无地址则弹出添加地址弹窗
 * @returns Promise，成功时设置addressList和formState.addressId
 */
const getAddressListData = async () => {
    try {
        const res = await getAddressList({ page: 1, size: 99 });
        if (res.records.length == 0) {
            // O2O 场景无地址不强弹（自提/预约到店无需地址；由 getOrderCheckData 后按 deliveryType 兜底触发）；非 O2O 场景保持强弹
            let shouldOpenDialog = true;
            if (shouldOpenDialog) {
                addressRef.value?.show();
            }
            return;
        }
        formState.addressId = res.records[0].addressId;
        addressList.value = res.records;
    } catch (e) {
        console.error("[order/check] 获取地址列表失败", e);
    }
};
/**
 * @description 新增/删除地址后刷新地址列表，并在存在地址时重新获取结算详情
 */
const addressRefresh = async () => {
    await getAddressListData();
    await getOrderCheckData();
};
/**
 * @description 设置用户选中的默认收货地址（服务端记录）
 * @param addressId 地址ID
 */
const setSelectedAddressData = async (addressId: number) => {
    try {
        await setSelectedAddress(addressId);
    } catch (e) {
        console.error("[order/check] 设置默认地址失败", e);
    }
};

const paymentTypeList = ref<PaymentTypeItem[]>([]);
/**
 * @description 获取可用的支付方式列表
 * 礼品卡订单使用特定flowType获取支付方式，默认选中第一个
 */
const getPaymentTypeData = async () => {
    try {
        const res = useGiftCard.value ? await getPaymentType(flowType.value) : await getPaymentType();
        formState.payTypeId = res[0].typeId;
        paymentTypeList.value = res;
    } catch (e) {
        console.error("[order/check] 获取支付方式失败", e);
    }
};

const shippingTypeData = ref<{ [key: string]: ShippingTypeItem[] }>({});
/**
 * @description 获取各店铺的配送方式列表，按店铺ID分组
 * @returns 无返回值，设置shippingTypeData和formState.shippingType
 */
const getShippingTypeData = async () => {
    try {
        const res = await getShippingType({ flowType: flowType.value });
        shippingTypeData.value = res;
        for (const key in res) {
            if (res[key] && res[key].length > 0) {
                formState.shippingType[key] = {
                    typeId: res[key][0].shippingTypeId,
                    shopId: res[key][0].shopId,
                    typeName: res[key][0].shippingTypeName
                };
            }
        }
    } catch (e) {
        console.error("[order/check] 获取配送方式失败", e);
    }
};

/**
 * @description 切换配送方式类型
 * @param shopId 店铺ID
 * @param data 配送方式数据
 */
const onChangeShippingType = async (shopId: number, data: ShippingTypeItem) => {
    const { shippingTypeId, shippingTypeName } = data;

    if (formState.shippingType[shopId] && formState.shippingType[shopId].typeId == shippingTypeId) return;

    formState.shippingType[shopId].typeId = shippingTypeId;
    formState.shippingType[shopId].shopId = shopId;
    formState.shippingType[shopId].typeName = shippingTypeName;

    await updateOrderCheckData();
};

const loading = ref(true);
const checkLoading = ref(true);
const useCoupon = ref(true);
const editAddressId = ref(0);
const showMoreAddress = ref(false);

const titleText = computed(() => {
    const arr = [commonStore.useCoupon == 1 && "优惠券", commonStore.usePoints == 1 && "积分", commonStore.useSurplus == 1 && "余额"].filter(Boolean);
    return arr.length ? `使用${arr.join("/")}` : "";
});

const cartList = ref<CheckCartList[]>([]);
const total = ref<CheckTotal>({} as CheckTotal);
const productPresale = ref<any>(null);
const couponList = ref<CheckCouponList>({ disableCoupons: [], enableCoupons: [] });
const availablePoints = ref(0);
const creditPaymentInfo = ref<CreditPaymentInfo | null>(null);

const creditPaymentInfoDisabled = computed(() => {
    return [2, 3].includes(creditPaymentInfo.value?.creditPaymentStatus as number);
});

const creditPaymentInfoDisabledText = computed(() => {
    if (creditPaymentInfo.value?.creditPaymentStatus === 2) {
        return t("您有账期逾期待结算订单，敬请及时处理");
    } else if (creditPaymentInfo.value?.creditPaymentStatus === 3) {
        return t("账期支付功能已关闭，详情请咨询平台方");
    } else {
        return "";
    }
});


const isDepositPresale = computed(() => {
    return productPresale.value?.saleMode === 2;
});

const payAmount = computed(() => {
    return isDepositPresale.value ? ((total.value as any)?.actualPayDepositAmount ?? total.value.unpaidAmount) : total.value.unpaidAmount;
});
/**
 * @description 获取订单结算页面完整数据，包括购物车商品、优惠券等
 * @returns 无返回值，设置cartList、total、couponList等状态
 */
const getOrderCheckData = async () => {
    checkLoading.value = true;
    try {
        const result = await getOrderCheck({ flowType: flowType.value, ...formState });
        if (result.item) {
            Object.assign(formState, result.item);
        }
        cartList.value = result.cartList;
        total.value = result.total;
        productPresale.value = (result as any).productPresale ?? null;
        couponList.value = result.couponList;
        availablePoints.value = result.availablePoints;
        creditPaymentInfo.value = result.creditPaymentInfo;

        if (result.useCouponIds && result.useCouponIds?.length > 0) {
            useCoupon.value = true;
            formState.useCouponIds = result.useCouponIds;
            formState.selectUserCouponIds = result.selectUserCouponIds!;
        }
        loading.value = false;
    } catch (error: any) {
        message.error(error.message);
    } finally {
        checkLoading.value = false;
    }
};

const LoadingBox = ref();
/**
 * @description 初始化结算页面数据，依次加载地址、支付方式、配送方式、用户信息
 * @returns 无返回值，完成所有基础数据加载后获取订单结算详情
 */
const initPageData = async () => {
    try {
        LoadingBox.value = ElLoading.service({
            lock: false,
            text: t("请求加载中"),
            customClass: "tig-normal-loading"
        });
        loading.value = true;
        await getAddressListData();
        await Promise.all([getPaymentTypeData(), getShippingTypeData(), userStore.getUserInfo()]);
        await getOrderCheckData();
    } catch (error: any) {
        console.error("[order/check] 结算页初始化失败", error);
    } finally {
        loading.value = false;
        LoadingBox.value?.close();
    }
};

initPageData();

const changeType = (data: any) => {
    if (data.type == "invoiceType") {
        formState.invoiceData.invoiceType = data.value;
    } else {
        formState.invoiceData.titleType = data.value;
    }
};
/**
 * @description 获取发票信息，若用户已有发票配置则回填，否则从地址信息预填充
 * @returns 无返回值，设置formState.invoiceData
 */
const fetcCheckInvoice = async () => {
    try {
        const result = await getCheckInvoice({ ...formState.invoiceData });
        if (result) {
            delete result.id;
            delete result.userId;
            delete result.orderId;
            delete result.status;
            delete result.applyReply;
            delete result.invoiceNo;
            delete result.addTime;
            if (result.titleType == 1) {
                result.companyUserName = result.companyName;
            }
            if (result.titleType == 2) {
                result.companyEnterpriseName = result.companyName;
            }
            formState.invoiceData = result;
        } else {
            let address = addressList.value.find((item: any) => formState.addressId == item.addressId);
            if (address) {
                let temp: any = {
                    companyUserName: address.consignee,
                    mobile: address.mobile,
                    email: address.email,
                    amount: total.value.unpaidAmount
                };
                Object.assign(formState.invoiceData, temp);
            }
        }
    } catch (error: any) {
        message.error(error.message);
    }
};

/**
 * @description 更新订单结算数据（当用户修改地址、支付方式等时调用）
 * @returns 更新后的结算数据
 */
const updateOrderCheckData = async () => {
    checkLoading.value = true;
    try {
        const result = await updateOrderCheck({ flowType: flowType.value, ...formState });
        if (result.item) {
            Object.assign(formState, result.item);
        }
        cartList.value = result.cartList;
        total.value = result.total;
        productPresale.value = (result as any).productPresale ?? null;
        availablePoints.value = result.availablePoints;
        return result;
    } catch (error: any) {
        message.error(error.message);
    } finally {
        checkLoading.value = false;
    }
};

const swiperOption = ref({
    autoplay: false,
    navigation: true,
    pagination: false,
    effect: "slide",
    loop: false,
    slidesPerView: 4, // 一屏显示的slide个数  'auto'
    slidesPerGroup: 1 //每组多少个swiper滑块
});

/**
 * @description 切换收货地址，更新相关配送和支付选项
 * @param e 地址对象，包含addressId
 */
const onChangeAddress = async (e: any) => {
    try {
        checkLoading.value = true;
        formState.addressId = e.addressId;
        await setSelectedAddressData(e.addressId);
        showMoreAddress.value = false;
        await getAddressListData();
        await getShippingTypeData();
        await getPaymentTypeData(), await updateOrderCheckData();
        fetcCheckInvoice();
    } catch (error) {
        console.error("[order/check] 切换收货地址失败", error);
    }
};
/**
 * @description 切换支付方式
 * @param typeId 支付方式ID
 */
const onChangePayType = (typeId: any) => {
    if (formState.payTypeId === typeId) return;
    formState.payTypeId = typeId;
    updateOrderCheckData();
};

const inputPoint = ref(0);
const showUsePoint = ref(false);
const getInvoiceData = (data: any) => {
    formState.invoiceData = data;
};
// const onUsePoint = () => {
//     if (showUsePoint.value === true) {
//         showUsePoint.value = false;
//         formState.usePoint = 0;
//         inputPoint.value = 0;
//     } else {
//         showUsePoint.value = true;
//     }
//     if (formState.usePoint == 0) return;
//     updateOrderCheckData();
// };
/**
 * @description 积分输入值变化处理，限制范围在0到可用积分之间
 */
const usePointChange = () => {
    inputPoint.value = Math.floor(Number(inputPoint.value));
    if (inputPoint.value > availablePoints.value) {
        inputPoint.value = availablePoints.value;
    } else if (inputPoint.value < 0 || !inputPoint.value) {
        inputPoint.value = 0;
    }
    formState.usePoint = inputPoint.value;
    updateOrderCheckData();
};

/**
 * @description 切换是否使用余额抵扣
 */
const onUseBalance = () => {
    if (+userStore.userInfo.balance === 0) return;
    const next = formState.useBalance == 0 ? Number(userStore.userInfo.balance || 0) : 0;
    formState.useBalance = next;
    // 与账期互斥：启用余额时关闭账期
    if (next > 0) {
        formState.useCredit = 0;
    }
    updateOrderCheckData();
};

/**
 * @description 切换是否使用账期支付（与余额互斥）
 */
const handleCredit = () => {
    if (!creditPaymentInfo.value?.canUseCredit) return;
    // 可用额度为 0 时不允许开启（保险兜底，正常由 canUseCredit 控制）
    if (Number(creditPaymentInfo.value.availableAmount || 0) <= 0 && formState.useCredit === 0) return;

    const next: 0 | 1 = formState.useCredit === 1 ? 0 : 1;
    formState.useCredit = next;
    // 与余额互斥：启用账期时关闭余额
    if (next === 1) {
        formState.useBalance = 0;
    }
    updateOrderCheckData();
};

/**
 * @description 优惠券选择变化处理
 * @param value 是否使用默认优惠券
 */
const onCouponChange = (value: boolean = false) => {
    updateCoupon(value);
};
const couponLoading = ref(false);
/**
 * @description 更新优惠券选择并重新计算订单金额
 * @param value 是否使用默认推荐的优惠券组合
 * @returns 更新后的优惠券和金额数据
 */
const updateCoupon = async (value: boolean = false) => {
    try {
        couponLoading.value = true;
        formState.useDefaultCouponIds = value ? 1 : 0;
        const result = await updateCouponData(formState);
        formState.useCouponIds = result.useCouponIds ?? [];
        formState.selectUserCouponIds = result.selectUserCouponIds ?? [];
        couponList.value = result.couponList;
        total.value = result.total;
        cartList.value = result.cartList;
        productPresale.value = (result as any).productPresale ?? null;
        availablePoints.value = result.availablePoints;
        return result;
    } catch (error) {
        console.error("[order/check] 更新优惠券失败", error);
    } finally {
        couponLoading.value = false;
    }
};
const showJoining = computed(() => {
    const activityInfo = (cartList.value?.[0]?.carts?.[0] as any)?.activityInfo;
    return Array.isArray(activityInfo) && activityInfo.find((item: any) => item.type === 7) ? true : false;
});
// 是否需要配送（只要有一个店铺需要配送，就展示运费/快递信息）
const needShipping = computed(() => {
    const hasNeed = cartList.value?.some((store) => store.noShipping === 0) ?? false;
    return hasNeed;
});
const showTaxAmount = computed(() => {
    return Number(total.value?.taxRate ?? 0) > 0;
});
const submitLoading = ref(false);
/**
 * @description 提交订单，校验地址后提交并跳转至支付页或订单列表
 * @returns 订单提交结果或跳转
 */
const onSubmit = async () => {
    // 地址必填校验：默认必填；O2O 场景下自提（deliveryType=2）/预约到店（flowType=9）无需地址
    let needAddress = true;
    if (needAddress && addressList.value?.length == 0) {
        message.error(t("请选择收货地址"));
        return;
    }
    try {
        submitLoading.value = true;
        const result = await orderSubmit(formState);
        if (result.returnType == 2) {
            if (showJoining.value && result.grouponPromotionRecordId) {
                navigateTo(`/member/group/info?id=${result.grouponPromotionRecordId}`, { replace: true });
            } else {
                navigateTo(`/member/order/list`, { replace: true });
            }
            userStore.userInfoLoaded = false;
            await userStore.getUserInfo(); // 刷新余额
            return;
        }
        navigateTo(`/order/pay?id=${result.orderId}`, { replace: true });
        return result;
    } catch (error: any) {
        message.error(error.message);
    } finally {
        submitLoading.value = false;
    }
};

const editAddressShow = ref<boolean>(false);

/**
 * @description 判断提交按钮是否禁用，检查地址和支付方式等必填项
 */
const submitDisabled = computed(() => {
    // 1. 检查收货地址（O2O 自提/预约到店场景无需地址）
    let needAddress = true;
    if (needAddress && formState.addressId === 0) {
        return true;
    }

    // 2. 检查支付方式
    if (formState.payTypeId === 0) {
        return true;
    }

    if (!formState.shippingType) {
        return true;
    }

    for (const item of cartList.value) {
        if (item.noShipping === 0 && !formState.shippingType[item.shopId]) {
            return true;
        }
    }

    return false;
});

const activityList = (data: any): any[] => {
    return data.filter((item: any) => item.type === 1 || item.type === 6);
};

const showPopover = ref(false);
</script>
<style lang="less">
.flex_csb {
    display: flex;
    justify-content: space-between;
}
body {
    background-color: #f5f6fa;
}
.order-check-page > .container {
    margin-top: 30px;
    padding: 0 var(--pc-home-padding-x, 38px);
}
@import "./css/check.less";
</style>
<style lang="less" scoped>
.wholesale-exclude-tip {
    color: #dd6b20;
    background: #fff8f1;
    border: 1px solid #ffd9b3;
    border-radius: 3px;
    padding: 8px 10px;
    margin-bottom: 12px;
    font-size: 12px;
    line-height: 1.6;

    div + div {
        margin-top: 4px;
    }

    .tip-icon {
        display: inline-block;
        width: 14px;
        height: 14px;
        border-radius: 50%;
        background: currentColor;
        color: #fff;
        text-align: center;
        line-height: 14px;
        font-size: 11px;
        font-weight: bold;
        margin-right: 4px;
        vertical-align: middle;
    }
}

/* O2O 自提信息（deliveryType=2 时展示于商品清单里） */
.pickup-box {
    margin-top: 16px;

    .pickup-card {
        background: #f7f8fa;
        border: 1px solid #f0f0f0;
        border-radius: 8px;
        padding: 16px 20px;

        .pickup-card-header {
            display: flex;
            justify-content: space-between;
            align-items: center;
            margin-bottom: 10px;

            .pickup-card-title {
                color: #1a1a1a;
                font-size: 16px;
                font-weight: 700;
            }
        }

        .pickup-card-body {
            border-bottom: 1px solid #e9e9e9;
            cursor: pointer;
            padding-bottom: 12px;

            .pickup-point-title {
                color: #1a1a1a;
                font-size: 16px;
                font-weight: 600;
                line-height: 22px;
                word-break: break-word;
            }
        }

        .pickup-card-footer {
            align-items: center;
            display: flex;
            gap: 16px;
            justify-content: space-between;
            padding-top: 12px;

            .pickup-time-text {
                color: #666;
                flex: 1;
                font-size: 14px;
                line-height: 20px;

                .label {
                    color: #666;
                    flex: 1;
                    font-size: 14px;
                    line-height: 20px;
                }
            }
        }

        .pickup-btn {
            padding: 6px 16px;
            border: 1px solid #dcdfe6;
            border-radius: 4px;
            background: #fff;
            font-size: 14px;
            line-height: 20px;
            color: #333;
            cursor: pointer;
            transition: all 0.2s;
            white-space: nowrap;
            user-select: none;
            outline: none;
            transition: border-color .2s, color .2s;

            &:hover {
                border-color: #c0c4cc;
            }
        }
    }
}

:deep(.el-checkbox__inner) {
    height: 18px;
    width: 18px;
    border: 2px solid #888b94;
    border-radius: 2.5px;
}
:deep(.el-checkbox__inner::after) {
    //top: -1.5px;
    //left: 29.5%;
    // right: 44%;
    width: 4.5px;
    height: 10.5px;
    border-width: 2px;
}

.extraskudata {
    margin-top: 5px;
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
        padding-bottom: 8px;

        .discounts-value {
            font-weight: bold;
        }

        .item-value {
            font-size: 14px;
        }
        &.hassub {
            display: block;
        }
        .sub-item-box {
            display: flex;
            align-items: center;
            justify-content: space-between;
            padding: 8px 0;
            color: #969799;
            font-size: 12px;
        }

        &:last-child {
            padding-top: 0;
        }
    }
    .border-bottom {
        border-bottom: 1px solid #e6e6e6;
        margin-bottom: 10px;
    }

    .discount-desc {
        font-size: 12px;
        color: #969799;
        letter-spacing: 0;
    }
}

.sku-name-box {
    display: flex;

    .sku-name-item {
        max-width: 110px;
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
        padding-left: 6px;
        color: #999;
        &:first-child {
            padding-left: 0;
        }
    }
}

.activity-box {
    display: flex;
    padding-top: 10px;
}

.promotion {
    display: flex;
    background-color: var(--tag-bg);
    color: var(--tag-text);
    border-radius: 2px;
    padding-bottom: 2px;
    .ticket-content {
        padding: 0 4px;
        max-width: 90px;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
    }
}

.gift-box {
    font-size: 14px;
    position: relative;
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
        grid-template-columns: repeat(2, 1fr);
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
                line-clamp: 2;
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
        display: inline-block;
        color: #969799;
        margin: 5px 0;

        .sku-item {
            display: inline-block;
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

.total_fee-box {
    flex: 1;
}
.total_fee {
    align-items: flex-end;

    :deep(.price) {
        height: 30px;
        line-height: 32px;
        align-items: baseline !important;
    }
}

.unpaidamount-label {
    height: inherit;
    display: flex;
    align-items: center;
    justify-content: center;
    font-weight: 700;
    font-size: 14px;
    margin-right: auto;
}
.points-box {
    display: flex;
    font-size: 26px;
    column-gap: 6px;
    color: var(--general);
    padding-right: 10px;
    height: 100%;
    .points-value {
        display: flex;
        align-items: flex-end;
        font-size: 26px;
        column-gap: 2px;
        // height: 30px;
        // line-height: 30px;
    }

    .points-text {
        font-size: 18px;
        position: relative;
        bottom: -4px;
    }
    .symbol {
        height: 30px;
        line-height: 36px;
    }
}
.item-label {
    font-size: 14px;
}

.tax-item-label {
    display: flex;
    align-items: center;
}

.tax-tip-icon {
    margin-left: 4px;
}

.tax-tip-content {
    line-height: 1.7;
    max-width: 260px;
}

@import "./css/check.less";
.flex_cc {
    display: flex;
    align-items: center;
    justify-content: center;
}
.ellipsis {
    font-size: 16px;
}
.buy-btn {
    border-radius: 4px;
    transition:
        background-color 0.3s ease,
        box-shadow 0.3s ease,
        transform 0.3s ease;
}
// 组件内部的 theme 悬停样式优先级更高，这里叠加权重保证下单按钮的悬浮反馈生效
.submit .buy-btn.btn:hover:not(:disabled) {
    color: var(--main-text);
    background-color: var(--primary-dark-2, var(--main-bg));
    border-color: var(--primary-dark-2, var(--main-bg));
    box-shadow: 0px 6px 16px rgba(0, 0, 0, 0.2);
    transform: translateY(-1px);
}
.pic {
    margin-right: 18px;
    img {
        width: 60px !important;
        height: 60px !important;
    }
}

.free-text {
    font-size: 14px;
    font-weight: 700;
    line-height: 16px;
    color: #269636;
    margin-left: 10px;
}

.discount-txt {
    background-color: var(--general);
    display: inline-block;
    font-size: 12px;
    line-height: 16px;
    color: #fff;
    border-radius: 2px;
    padding: 0 5px;
    margin-right: 6px;
    flex-shrink: 0;
}
:deep(.el-checkbox__inner) {
    width: 18px !important;
    height: 18px !important;
    border: 2px solid #e5e7eb;
    &::after {
        left: 50%;
        top: 50%;
        transform: translate(-45%, -60%) rotate(45deg) scaleY(1) !important;
    }
}

// 账期禁用时强制显示为未勾选状态
.credit-disabled {
    :deep(.el-checkbox) {
        .el-checkbox__inner {
            background-color: #f5f7fa !important;
            border-color: #e4e7ed !important;
            &::after {
                display: none !important;
            }
        }
        .el-checkbox__input.is-checked .el-checkbox__inner {
            background-color: #f5f7fa !important;
            border-color: #e4e7ed !important;
            &::after {
                display: none !important;
            }
        }
    }
}
// O2O 起送差价提示 / 运费规则弹窗 / 已超出配送范围提示
.card-tip {
    display: flex;
    align-items: center;
    column-gap: 2px;
    color: #ff4d4f;
    font-size: 12px;
    margin-top: -5px;
    padding-bottom: 5px;
    .icon-tishi {
        font-size: 16px;
    }
}
.freightraterules-box {
    display: flex;
    flex-direction: column;
    row-gap: 10px;
    color: #333;
    .freightraterules-title {
        font-size: 16px;
        font-weight: bold;
        text-align: left;
        background: #fff;
    }
    .shippingfee-item {
        border-radius: 4px;
        background-color: #f4f4f4;
        padding: 12px 10px;
        color: #9a9a9a;
        font-size: 12px;
        display: flex;
        flex-direction: column;
        row-gap: 10px;
        text-align: left;
        .store-title {
            font-weight: bold;
            font-size: 14px;
            color: #333;
        }
        .delivery-rules {
            display: flex;
            flex-direction: column;
            row-gap: 10px;
            .rule-item {
                display: flex;
                width: 100%;
                span {
                    word-break: keep-all;
                    line-height: 36rpx;
                }
            }
        }
    }
}
.submit {
    .tips {
        color: #ff4d4f;
        margin-top: 10px;
        text-align: right;
        font-size: 12px;
    }
}
</style>
