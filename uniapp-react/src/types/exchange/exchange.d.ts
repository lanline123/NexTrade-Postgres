export interface ExchangeListResponse {
    data: ExchangeListData;
    code: number;
    message: string;
}

export interface ExchangeListData {
    records: ExchangeListFilterResult[];
    filter: ExchangeListFilter;
    total: number;
}

export interface ExchangeListFilter {
    isEnabled: number;
    page: number;
    size: number;
    sortField: string;
    sortOrder: string;
}

export interface ExchangeListFilterResult {
    id: number;
    productId: number;
    exchangeIntegral: number;
    pointsDeductedAmount: string;
    isHot: number;
    isEnabled: number;
    skuId: number;
    productName: string;
    marketPrice: string;
    productPrice: string;
    picUrl: string;
    virtualSales: number;
    ProductSn: string;
    productStock: number;
    discountsPrice: string;
    productSku: ExchangeListProductSku | null;
    // 券型兑换品字段（我方新增）：exchangeType=2 时商品字段为空，
    // exchangeIntegral 已是生效价（自设或平台统一比折算）
    exchangeType?: number;
    couponId?: number;
    couponName?: string;
    couponMoney?: string;
    couponDiscount?: string;
    minOrderAmount?: string;
    couponShopId?: number;
    integralSource?: string;
}

export interface ExchangeListProductSku {
    skuId: number;
    productId: number;
    skuValue: string;
    skuData: ExchangeListSkuDatum[];
    skuSn: string;
    skuStock: number;
    skuTsn: string;
    skuPrice: string;
}

export interface ExchangeListSkuDatum {
    name: string;
    value: string;
}
