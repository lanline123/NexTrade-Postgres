// 列表查询时筛选参数类型
import type { AfterSalesFormState, PicsList } from "~/types/user/afterSales";

export interface AfterSalesRecordFilterParams {
    page: number;
    size?: number;
}

// 获取列表返回参数类型
export interface AfterSalesRecordFilterState {
    aftersaleId: number;
    aftersalesSn: string;
    aftersalesItems: AfterSalesRecordFormState[];
    status: number;
    productId: number;
    orderId: number;
    orderSn: string;
    picThumb: string;
    productSn: string;
    productName: string;
    number: number;
    addTime: string;
    aftersalesTypeName: string;
    statusName: string;
}

export interface AfterSalesRecordFilterResult {
    records: AfterSalesRecordFilterState[];
    filter: AfterSalesRecordFilterParams;
    total: number;
}

// 获取详情返回参数类型
export interface AfterSalesRecordFormState {
    aftersaleId?: number;
    productId?: number;
    canCancel?: boolean;
    status?: number;
    number?: number;
    productSn?: string;
    refundAmount?: string;
    picThumb?: string;
    logInfo?: string;
    productName?: string;
    returnAddress?: string;
    logisticsName?: string;
    trackingNo?: string;
    aftersaleReason?: string;
    pics: PicsList[];
    picsList: PicsList[];
    returnPic: PicsList[];
    addTime?: string;
    aftersalesLog?: AftersalesLog[];
    aftersalesItems?: AfterSalesFormState[];
    description?: string;
    aftersalesTypeName?: string;
    stepStatus: StepStatusItem;
    returnGoodsTip?: string;
}

export interface StepStatusItem {
    current?: number;

    status?: string;
    steps?: StepsArray[];
}

export interface StepsArray {
    description: string;
    active?: boolean;
    title: string;
}

export interface AftersalesLog {
    adminName?: string;
    returnPic?: PicsList[];
    userName?: string;
    addTime?: string;
    logInfo?: string;
    pics?: string;
}

// export interface AfterSalesRecordFormResult {
//     item: AfterSalesRecordFormState;
//     aftersalesItems: AfterSalesRecordFormState[];
// }

// 编辑表单
