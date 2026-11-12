interface OrderPaymentState {
    availableActions?: { toPay?: boolean };
    payTypeId?: number | string | null;
    offlinePaymentClaimStatus?: number | string | null;
    presaleInfo?: { saleMode?: number | string; presaleStatus?: number | string } | null;
}

/** 认款通过的定金订单仍可支付尾款，支付权限继续以后端返回为准。 */
export const canShowOrderPayment = (order: OrderPaymentState): boolean => {
    if (!order.availableActions?.toPay) return false;
    if (Number(order.payTypeId) !== 3) return true;

    const status = order.offlinePaymentClaimStatus;
    if (status === null || status === undefined || Number(status) === 2) return true;

    return Number(status) === 1 && Number(order.presaleInfo?.saleMode) === 2 && Number(order.presaleInfo?.presaleStatus) === 1;
};
