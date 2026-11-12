import { request, asyncRequest } from "@/utils/request";
import type { OrderPayParams, OrderPayInfo } from "~/types/order/pay";

/**
 * 获取订单支付信息
 * @description 进入支付页时获取订单金额、可用支付方式等
 * @param params - 订单参数（订单号等）
 * @returns 支付页所需数据
 */
export const orderPayInfo = (params: OrderPayParams) => {
    return request<OrderPayInfo>({
        url: "order/pay/index",
        method: "get",
        params
    });
};

/**
 * 创建支付请求
 * @description 选择支付方式后发起支付，返回支付参数或跳转地址
 * @param params - 支付参数（订单号、支付方式等）
 * @returns 支付创建结果
 */
export const creatPay = (params: {}) => {
    return asyncRequest({
        url: "order/pay/create",
        method: "get",
        params
    });
};

/**
 * 创建定金支付请求
 * @description 预售定金支付场景使用，返回定金支付参数或跳转内容
 * @param params - 支付参数（订单号、支付方式等）
 * @returns 定金支付创建结果
 */
export const createDepositPayment = (params: {}) => {
    return asyncRequest({
        url: "order/pay/createDepositPayment",
        method: "get",
        params
    });
};

/**
 * 检查支付状态
 * @description 轮询检查订单是否支付成功
 * @param params - 订单参数
 * @returns 支付状态结果
 */
export const checkPayStatus = (params: OrderPayParams) => {
    return asyncRequest<number>({
        url: "order/pay/checkStatus",
        method: "get",
        params
    });
};

/**
 * @description Apple Pay 商户验证（原生 ApplePaySession.onvalidatemerchant 阶段调用）
 * @param data 包含订单 id 与 Apple 下发的 validationURL
 * @returns 商户会话对象，用于 completeMerchantValidation
 */
export const applePayValidateMerchant = (data: { id: any; validationURL: string }) => {
    return asyncRequest({
        url: "order/pay/applepay/validateMerchant",
        method: "post",
        data
    });
};

/**
 * @description Apple Pay 支付确认（原生 ApplePaySession.onpaymentauthorized 阶段调用）
 * @param data 包含订单 id 与 Apple 返回的支付 token（序列化后的 payment.token）
 * @returns 后端处理结果
 */
export const applePayConfirm = (data: { id: any; token: string }) => {
    return asyncRequest({
        url: "order/pay/applepay/confirm",
        method: "post",
        data
    });
};

/**
 * 获取账期结算支付信息
 * @description 获取线下支付方式列表（银行汇款、企业汇款等）
 * @returns 线下支付方式信息
 */
export const orderPayInfoTerm = () => {
    return asyncRequest<Pick<OrderPayInfo, "offlinePaymentList">>({
        url: "order/pay/index",
        method: "get"
    });
};