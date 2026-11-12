export interface CheckResponse {
    errcode: number;
    message: string;
    data: CheckItem;
}

/** 账期支付信息（GET /api/order/check/index 新增字段） */
export interface CreditPaymentInfo {
    creditPaymentStatus: number;
    canUseCredit: boolean;
    disableReason: string | null;
    creditLimit: number;
    usedAmount: number;
    availableAmount: number;
    settlementType: number;
    settlementTypeText: string;
    settlementDays: number;
    remainingDays: number;
}

export interface CheckItem {
    cartList: CheckCartList[];
    total: CheckTotal;
    couponList: CheckCouponList;
    item: CheckItemItem;
    useCouponIds: null | number[];
    selectUserCouponIds: null | number[];
    tmplIds: string[];
    flowType: number;
    availablePoints: number;
    deliveryOption?: any;
    message: null;
    creditPaymentInfo?: CreditPaymentInfo | null;
    /** 预售配置（结算维度），存在时表示本单为预售相关结算 */
    productPresale?: any;
}

export interface StockCalendarParams {
    productId: number;
    shopId: number;
    skuId: number;
}

export interface StockCalendarTimeSlotItem {
    productReservationTimeId: number;
    startTime: string;
    endTime: string;
    availableStock: number;
}

export interface StockCalendarDateItem {
    stockDate: string;
    timeSlotStockList: StockCalendarTimeSlotItem[];
}

export interface StockCalendarResult {
    dateStockList: StockCalendarDateItem[];
}

export interface CheckCouponList {
    enableCoupons: EnableCoupon[];
    disableCoupons: any[];
}

export interface EnableCoupon {
    id: number;
    couponName: string;
    couponType: number;
    minOrderAmount: string;
    couponDesc: string;
    couponMoney: string;
    isGlobal: number;
    couponDiscount: string;
    shopId: number;
    endDate: string;
    couponId: number;
    userCouponId: number;
    selected?: boolean;
}

export interface CheckCartList {
    total: CartListTotal;
    carts: Cart[];
    usedPromotions: any[];
    enableUsePromotion: any[];
    gift: any[];
    shopId: number;
    shopTitle: string;
    noShipping: number;
    minOrderAmountDiff: string;
    isNotReachMinOrderAmount: boolean;
}

export interface Cart {
    cartId: number;
    userId: number;
    productId: number;
    productSn: string;
    seoLink?: string;
    picThumb: string;
    marketPrice: null;
    originalPrice: null;
    quantity: number;
    skuId: number;
    skuData: any[];
    productType: number;
    isChecked: boolean;
    shopId: number;
    type: number;
    updateTime: number;
    salesmanId: number;
    extraSkuData: null | any[];
    productWeight: null;
    shippingTplId: null;
    freeShipping: number;
    productStatus: number;
    productName: string;
    productPrice: number;
    categoryId: number;
    brandId: number;
    productStock: number;
    cardGroupId: number;
    virtualSample: string;
    suppliersId: number;
    shop: Shop;
    sku: null;
    price: number;
    stock: number;
    hasSku: boolean;
    subtotal: number;
    originPrice: number;
    isDisabled: boolean;
    activityInfo: null;
    serviceFee: null;
    isSeckill: null;
    /** 预售标识：存在时商品为预售（用于列表/结算标记） */
    productSaleMode?: number;
}

export interface Shop {
    description: string;
    kefuInlet: null | string;
    kefuLink: string;
    kefuPhone: string;
    kefuWeixin: string;
    merchantId: number;
    shopId: number;
    shopLogo: string;
    shopTitle: string;
    status: number;
}

export interface CartListTotal {
    couponIds: any[];
    discountCouponAmount: number;
    discountProductPromotionAmount: number;
    discountSeckillAmount: number;
    discountTimeDiscountAmount: number;
    discountDiscountAmount: number;
    discounts: number;
}

export interface CheckItemItem {
    addressId: number;
    shippingType: ShippingType[];
    payTypeId: number;
    usePoint: number;
    useBalance: number;
    /** 是否使用账期支付：0-不使用，1-使用（与余额互斥） */
    useCredit?: 0 | 1;
    useCouponIds: null;
    selectUserCouponIds: null;
    flowType: number;
    productReservationTimeId?: number;
    reservationDate?: string;
    deliveryType?: number;
}

export interface ShippingType {
    typeId: null;
    shopId: null;
    typeName: null;
}

export interface CheckTotal {
    productAmount: number;
    checkedCount: number;
    discounts: number;
    discountAfter: null;
    totalCount: number;
    discountCouponAmount: number;
    discountDiscountAmount: number;
    discountSeckillAmount: number;
    discountProductPromotionAmount: number;
    discountTimeDiscountAmount: number;
    serviceFee: number;
    paidAmount: number;
    couponAmount: number;
    discountAmount: number;
    exchangePoints: number;
    pointsAmount: number;
    shippingFee: number;
    storeShippingFee: StoreShippingFee;
    totalAmount: number;
    orderSendPoint: number;
    balance: number;
    unpaidAmount: number;
    storeShippingRule?: any;
    /** 定金预售首付款（应付定金） */
    actualPayDepositAmount?: number;
    /** 税率（跨境专用；> 0 时展示税费行） */
    taxRate?: number;
    /** 税费金额（跨境专用） */
    taxRateAmount?: number;
    /** 礼品卡抵扣金额（标准版 flowType=12 场景，>0 时展示「使用礼品卡」行） */
    giftCardAmount?: number;
}

export interface StoreShippingFee {
    [key: string]: number;
}

export interface PaymentTypeResponse {
    errcode: number;
    message: string;
    item: PaymentTypeItem[];
}

export interface PaymentTypeItem {
    typeId: number;
    typeName: string;
    disabled: boolean;
    disabledDesc: string;
    isShow: boolean;
}

export interface ShippingTypeResponse {
    errcode: number;
    message: string;
    item: { [key: string]: ShippingTypeItem[] };
}

export interface ShippingTypeItem {
    shippingTypeId: number;
    shippingTypeName: string;
    shippingDefaultId: number;
    shippingTypeDesc: string;
    shippingTimeDesc: string;
    isSupportCod: number;
    sortOrder: number;
    shopId: number;
}
