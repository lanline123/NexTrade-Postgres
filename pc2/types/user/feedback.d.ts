import type { BaseResponseListWrap } from "~/types/api";
// 列表查询时筛选参数类型
export interface FeedbackFilterParams {
    page: number;
    size?: number;
    orderId?: number;
    isOrder?: number;
}

// 获取列表返回参数类型
export interface FeedbackFilterState {
    type?: number;
    product?: {
        productId?: number;
        productName?: string;
    };
    typeName?: string;
    orderSn?: string;
    title?: string;
    content?: string;
    addTime?: string; // 或者 Date 类型，取决于实际数据格式
    reply?: ReplyState;
}

export interface ReplyState {
    addTime: string; // 或者 Date 类型
    content: string;
}

export interface FeedbackFilterResult extends BaseResponseListWrap<FeedbackFilterState[]> {
    filter: FeedbackFilterParams;
}

// 获取详情返回参数类型
export interface FeedbackFormState {
    type?: number;
    orderId?: number;
    title?: string;
    content?: string;
}

// 编辑表单
