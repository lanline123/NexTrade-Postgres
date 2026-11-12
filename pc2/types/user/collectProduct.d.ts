import type { SkuList } from "~/types/product/product";
import type { BaseResponseListWrap } from "~/types/api";
// 列表查询时筛选参数类型
export interface CollectProductFilterParams {
    page: number;
    size?: number;
    keyword?: string;
    sortField?: string;
    sortOrder?: string;
}

// 获取列表返回参数类型
export interface CollectProductFilterState {
    collectId: number;
    productStock: number;
    userId?: number;
    productId: number;
    addTime?: string;
    isAttention?: number;
    productName?: string;
    productSn?: string;
    picThumb: string;
    marketPrice?: string;
    isPromote?: number;
    promotePrice?: string;
    promoteStartDate?: string;
    promoteEndDate?: string;
    username?: string;
    rankId?: number;
    discount?: number;
    userPrice?: string;
    productPrice?: string;
    productSku?: SkuList[];
    price?: string;
}

export interface CollectProductFilterResult extends BaseResponseListWrap<CollectProductFilterState[]> {
    // filter: CollectProductFilterParams;
}

// 获取详情返回参数类型
export interface CollectProductFormState {
    type?: number;
    title?: string;
    content?: string;
}

// 编辑表单
