/**/ // 列表查询时筛选参数类型
export interface CartFilterParams {
    page: number;
    size: number;
    sortField: string;
    sortOrder: string;
    keyword?: string;
    payStatus?: number;
    orderStatus?: number;
    addTime?: string[];
    shippingStatus?: number;
    commentStatus?: number;
}

// 获取列表返回参数类型
export interface CartFilterState {
    shopId: number;
    shopTitle: string;
    isChecked: boolean;
    carts: CartItmes[];
    gift: any[];
}

export interface CartItmes {
    activityInfo: any;
    cartId: number;
    userId?: number;
    productId?: number;
    productSn?: string;
    seoLink?: string;
    productName?: string;
    picThumb?: string;
    marketPrice?: string;
    originalPrice?: string;
    quantity?: number;
    skuId?: number;
    skuData?: SkuData[];
    productType?: number;
    isChecked?: boolean;
    /** 当前销售门店；批发 SPU 聚合必须与 productId 共同作为作用域 */
    shopId?: number;
    type?: number;
    shopTitle?: string;
    productWeight?: string;
    shippingTplId?: number;
    freeShipping?: number;
    productStatus?: number;
    saleStatus?: number;
    effectiveSaleStatus?: number;
    price?: string;
    stock?: number;
    isDisabled?: boolean;
    sku?: { isEnabled?: number | null };
    subtotal?: number;
    extraSkuData: any[];
    limitNumber?: number;
}

export interface SkuData {
    name: string;
    value: string;
}

export interface CartFilterResult {
    cartList: CartFilterState[];
    total: TotalObj;
}

export interface TotalObj {
    productAmount: number;
    checkedCount: number;
    discounts: number;
    discountAfter: number;
    totalCount: number;
    discountCouponAmount: number;
    discountSeckillAmount: number;
    discountProductPromotionAmount: number;
    discountTimeDiscountAmount: number;
    discountDiscountAmount: number;
    serviceFee?: number;
}

// 获取详情返回参数类型
export interface CartFormState {
    orderId?: number;
    orderStatus?: number;
    shopId?: number;
    payTypeId?: number;
    orderSn?: string;
    addTime?: string;
}

export interface CartFormResult {
    item: CartFormState;
}

// 编辑表单
