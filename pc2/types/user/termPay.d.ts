export interface UserCreditOrderGoodItem {
    productId: number;
    productName: string;
    picThumb: string;
    quantity: number;
    /** 后端可能会返回价格字段（不同端口径不完全一致），这里做可选兜底 */
    price?: string | number;
    skuValue?: string;
}

export interface UserCreditOrderItem {
    orderId: number;
    orderSn: string;
    orderTime: string;
    /** 账期付款金额：当前口径为“剩余待结算金额” */
    creditAmount: string | number;
    totalQuantity: number;
    /** 0-未结算 1-已结算 2-已取消 3-退款中 */
    settlementStatus: number;
    settlementStatusText: string;
    /** 0-正常 1-逾期 */
    riskStatus: number;
    riskStatusText: string;
    items: UserCreditOrderGoodItem[];
}

export interface UserCreditDetailVO {
    creditLimit: string | number;
    usedAmount: string | number;
    availableAmount: string | number;
    /** 1-月结 2-季结 */
    settlementType: number;
    /** 30 或 90 */
    settlementDays: number;
    remainingDays: number;
    pendingOrders: UserCreditOrderItem[];
    settledOrders: UserCreditOrderItem[];
}

