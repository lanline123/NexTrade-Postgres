import type { BaseResponseListWrap } from "~/types/api";
// 列表查询时筛选参数类型
export interface CollectShopFilterParams {
    page: number;
    size?: number;
    keyword?: string;
}

// 获取列表返回参数类型
export interface CollectShopFilterState {
    addTime: string;
    current: string;
    collectId: number;
    shopId: number;
    userId: number;
    page: number;
    id: number;
    items: items[];
    shop: Shop;
}
interface items {
    text: string;
}
export interface Shop {
    shopId: number;
    addTime: string;
    shopTitle: string;
    shopLogo: string;
    kefuLink: string;
    bestProduct: Product[];
    hotProduct: Product[];
    newProduct: Product[];
}
interface Product {
    picUrl: string;
    productName: string;
    productPrice: string;
    productSn: string;
    productId: number;
    seoLink?: string;
}

export interface CollectShopFilterResult extends BaseResponseListWrap<CollectShopFilterState[]> {
}
