export interface ExchangeFilterState {
    id: number;
    productId: number;
    discountsPrice: number;
    exchangeIntegral: number;
    isEnabled: number;
    isHot: number;
    marketPrice: number;
    productPrice: number;
    picUrl: string;
    pointsDeductedAmount: number;
    productName: string;
    productSn: string;
    virtualSales: number;
}

export interface ExchangeFilterParams {
    page: number;
    size: number;
    sortField: string;
    sortOrder: string;
    isEnabled: number;
}

export interface ExchangeFilterResult {
    records: ExchangeFilterState[];
    filter: ExchangeFilterParams;
    total: number;
}
