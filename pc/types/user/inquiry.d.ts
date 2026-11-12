import type { BaseResponseListWrap } from "~/types/api";

export interface InquiryFilterParams {
    page: number;
    size?: number;
    status?: number;
    productId?: number;
    sortField?: string;
    sortOrder?: string;
}

export interface InquiryProduct {
    productId?: number;
    productName?: string;
    productSn?: string;
    picThumb?: string;
    marketPrice?: string | number;
    productPrice?: string | number;
}

export interface InquiryReplyItem {
    replyName?: string;
    createTime?: string;
    content?: string;
    replyType?: number;
}

export interface InquiryItem {
    id?: number;
    product?: InquiryProduct;
    content?: string;
    status?: number;
    remark?: string;
    createTime?: string;
}

export interface InquiryFilterResult extends BaseResponseListWrap<InquiryItem[]> {
    filter: InquiryFilterParams;
}

export interface InquiryDetail {
    id?: number;
    product?: InquiryProduct;
    content?: string;
    createTime?: string;
    contactName?: string;
    mobile?: string;
    remark?: string;
    status?: number;
    replyRecords?: InquiryReplyItem[];
}

export interface InquiryReplyFormState {
    id?: number;
    remark?: string;
}
