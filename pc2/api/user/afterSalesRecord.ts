import type { AfterSalesRecordFilterParams, AfterSalesRecordFilterResult, AfterSalesRecordFormState } from "~/types/user/afterSalesRecord";

export const getAfterSalesRecordList = (params: AfterSalesRecordFilterParams) => {
    return asyncRequest<AfterSalesRecordFilterResult>({
        url: "user/aftersales/getRecord",
        method: "get",
        params
    });
};
//
export const getAfterSalesRecord = (params: object) => {
    return asyncRequest<AfterSalesRecordFormState>({
        url: "user/aftersales/detail",
        method: "get",
        params
    });
};
export const updateAfterSalesRecord = (data: object) => {
    return asyncRequest<AfterSalesRecordFormState>({
        url: "user/aftersales/feedback",
        method: "post",
        data
    });
};
//
export const cancelAfterSalesRecord = (data: object) => {
    return asyncRequest<AfterSalesRecordFormState>({
        url: "user/aftersales/cancel",
        method: "post",
        data
    });
};
