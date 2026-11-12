import type { OrderFormState } from "~/types/user/order";

export interface OrderPayParams {
    id: number | string | null | (string | null)[];
    paylogId?: number;
}

export interface OfflinePaymentList {
    thirdPartyPaymentTransfer?: string;
    offlinePayBank?: string;
    offlinePayCompany?: string;
    otherPaymentRemittance?: string;
}

export interface OrderPayInfo {
    order: OrderFormState;
    paymentList: string[];
    offlinePaymentList: OfflinePaymentList;
    /** 兼容现有页面的拼团跳转字段；不是所有部署都返回。 */
    grouponPromotionRecordId?: number;
}
