import { asyncRequest, request } from "@/utils/request";
import type { OrderFilterParams, OrderFilterResult, OrderFormResult, OrderNumResult, ShippingInfoResponse, PickupInfoData, OrderFormState, OfflinePaymentClaim, OfflinePaymentClaimSubmitParams } from "~/types/user/order.d";

/**
 * 获取订单列表
 * @description 分页获取用户订单列表，支持按状态筛选
 * @param params - 筛选和分页参数
 * @returns 订单列表
 */
export const getOrderList = (params: OrderFilterParams) => {
    return asyncRequest<OrderFilterResult>({
        url: "user/order/list",
        method: "get",
        params
    });
};

/**
 * 获取订单数量统计
 * @description 获取各状态订单数量（待支付、待发货等）
 * @returns 订单数量统计
 */
export const getOrderNum = () => {
    return asyncRequest<OrderNumResult>({
        url: "user/order/orderNum",
        method: "get"
    });
};

/**
 * 删除订单
 * @description 删除已取消或已完成的订单
 * @param data - 订单ID
 * @returns 操作结果
 */
export const delOrder = (data: object) => {
    return request({
        url: "user/order/delOrder",
        method: "post",
        data
    });
};
/**
 * 取消订单
 * @description 取消未支付的订单
 * @param data - 订单ID和取消原因
 * @returns 操作结果
 */
export const cancelOrder = (data: object) => {
    return request({
        url: "user/order/cancelOrder",
        method: "post",
        data
    });
};

export interface OrderReasonConfig {
    /** 原因开关，取消订单与售后共用 */
    afterSalesReasonEnabled: number;
    /** 取消未支付订单原因 */
    cancelOrderReasonList: string[];
    /** 已废弃，仅后端兼容旧前端保留，不要再读 */
    afterSalesReasonList?: string[];
}

/** 获取买家主动取消未支付订单使用的平台原因配置 */
export const getOrderReasonConfig = () => {
    return asyncRequest<OrderReasonConfig>({
        url: "user/order/reasonConfig",
        method: "get"
    });
};

/**
 * 确认收货
 * @description 用户确认已收到商品
 * @param data - 订单ID
 * @returns 操作结果
 */
export const confirmReceipt = (data: object) => {
    return request({
        url: "user/order/confirmReceipt",
        method: "post",
        data
    });
};

/**
 * 再次购买
 * @description 将历史订单商品重新加入购物车
 * @param data - 订单ID
 * @returns 操作结果
 */
export const reBuy = (data: object) => {
    return request({
        url: "user/order/buyAgain",
        method: "post",
        data
    });
};

/**
 * 获取订单详情
 * @description 获取订单完整信息
 * @param params - 订单ID
 * @returns 订单详情
 */
export const getOrder = (params: object) => {
    return request<OrderFormResult>({
        url: "user/order/detail",
        method: "get",
        params
    });
};
/**
 * 获取IM订单详情
 * @description 在线客服场景下获取订单详情
 * @param params - 订单ID
 * @returns 订单详情
 */
export const getImOrder = (params: object) => {
    return request<OrderFormState>({
        url: "user/order/detail",
        method: "get",
        params,

    });
};

/**
 * 更新订单操作
 * @description 执行订单相关操作（如修改地址等）
 * @param operation - 操作类型
 * @param data - 操作参数
 * @returns 操作结果
 */
export const updateOrder = (operation: string, data: object) => {
    return request({
        url: "user/order/" + operation,
        method: "post",
        data
    });
};

/**
 * 获取物流信息
 * @description 查询订单的物流轨迹
 * @param params - 订单ID
 * @returns 物流信息
 */
export const getShippingInfo = (params: object) => {
    return asyncRequest<ShippingInfoResponse>({
        url: "user/order/shippingInfo",
        method: "get",
        params
    });
};

/**
 * 获取自提/预约订单信息
 * @description O2O场景下获取订单的自提或配送预约信息
 * @param params - 订单ID
 * @returns 自提/预约信息
 */
export const getPickupInfo = (params?: object) => {
    return asyncRequest<PickupInfoData>({
        url: "o2o/orderPickup/getOrderPickupShipment",
        method: "get",
        params
    });
};

/** 提交线下支付认款申请 */
export const submitOfflinePaymentClaim = (data: OfflinePaymentClaimSubmitParams) => {
    return request<{ claimId: number }>({
        url: "user/order/offlinePaymentClaim/submit",
        method: "post",
        data
    });
};

/** 查询最新线下支付认款详情 */
export const getOfflinePaymentClaimDetail = (params: { orderId: number }) => {
    return asyncRequest<OfflinePaymentClaim | null>({
        url: "user/order/offlinePaymentClaim/detail",
        method: "get",
        params
    });
};
