import { asyncRequest } from "~/utils/request";
import type { InquiryDetail, InquiryFilterParams, InquiryFilterResult, InquiryReplyFormState } from "~/types/user/inquiry.d";

export const getInquiryList = (params: InquiryFilterParams) => {
    return asyncRequest<InquiryFilterResult>({
        url: "user/priceInquiry/list",
        method: "get",
        params
    });
};

export const getInquiryDetail = (id: number) => {
    return asyncRequest<InquiryDetail>({
        url: "user/priceInquiry/detail",
        method: "get",
        params: {
            id
        }
    });
};

export const replyInquiry = (data: InquiryReplyFormState) => {
    return asyncRequest({
        url: "user/priceInquiry/reply",
        method: "post",
        data
    });
};

export const deleteInquiry = (id: number) => {
    return asyncRequest({
        url: "user/priceInquiry/del",
        method: "post",
        data: {
            id
        }
    });
};
