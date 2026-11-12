import type { SkuList } from "~/types/product/product";

export interface GrouponItem {
    productId: number;
    productName: string;
    productSn?: string;
    seoLink?: string;
    picThumb?: string;
    picUrl: string;
    productBrief?: string;
    picLarge?: string;
    minGrouponPrice: string | number;
    productPrice: string | number;
    groupNum?: string;
    startTime: string;
    endTime: string;
    productStock?: number;
    productSku?: SkuList[];
}

export interface GrouponItemListParams {
    page?: number;
    size?: number;
    sortField?: string;
    keyword?: string;
    sortOrder?: string;
    grouponPromotionId?: string | number;
}

export interface GrouponItemListResult {
    records: GrouponItem[];
}
