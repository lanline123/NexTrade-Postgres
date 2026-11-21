import request from "@/utils/request";

/**
 * 骑手模式 API（自营骑手 MVP，我方新增前端；后端 o2o 人员/派送接口为原版有·我方补齐接线）。
 * 全部走 /api/o2o/** 人员域接口：身份由会员 token 解析绑定的 service_staff，
 * 大厅/动作的作用域（本店 + 同类型）在后端强制校验。
 */

/** 响应信封（与 @/utils/request 的 ResponseData 约束及 auction 等模块 *Result 口径一致）：
 * request<T extends ResponseData> 返回 Promise<T["data"]>，故泛型须传信封而非载荷本身。 */
type RiderEnvelope<T> = {
    code?: number;
    message: string;
    data: T;
};

export interface RiderInfo {
    id: number;
    name: string;
    nickname?: string;
    avatar?: string;
    phone: string;
    storeId: number;
    type: number; // 1 配送员 2 服务员
    status: number; // 0 禁用 1 启用
    workStatus: number; // 0 已下班 1 空闲 2 忙碌
    completedOrderCount?: number;
    completedOrderAmount?: number;
}

export interface DeliveryOrderRow {
    deliveryOrderId: number;
    deliveryOrderSn: string;
    orderId: number;
    orderSn: string;
    shopId: number;
    type: number;
    dispatchMode?: number;
    status: number;
    staffId?: number | null;
    staffName?: string;
    consignee: string;
    mobile: string;
    address?: string;
    regionNames?: string;
    latitude?: number | null;
    longitude?: number | null;
    pickupAddress?: string;
    pickupRegionNames?: string;
    pickupPhone?: string;
    orderAmount?: number;
    serviceFee?: number;
    buyerNote?: string;
    serviceTime?: number | null;
    createTime?: number;
    grabTime?: number | null;
    pickupTime?: number | null;
    completeTime?: number | null;
    cancelTime?: number | null;
    cancelReason?: string;
}

export const riderMyInfo = () => request<RiderEnvelope<RiderInfo | null>>({ url: "o2o/serviceStaff/myInfo", method: "get" });

export const riderUpdateWorkStatus = (workStatus: number) =>
    request({ url: "o2o/serviceStaff/updateWorkStatus", method: "post", data: { workStatus } });

export const riderStatistics = () =>
    request<RiderEnvelope<{ total: number; pending: number; processing: number; completed: number; cancelled: number }>>({
        url: "o2o/deliveryOrder/statistics",
        method: "get"
    });

export const riderGrabList = (params: { page: number; size: number; type?: number }) =>
    request<RiderEnvelope<{ records: DeliveryOrderRow[]; total: number; canTakeOrders?: boolean }>>({
        url: "o2o/deliveryOrder/grabList",
        method: "get",
        params
    });

export const riderGrab = (data: { deliveryOrderId: number; longitude?: number; latitude?: number }) =>
    request({ url: "o2o/deliveryOrder/grab", method: "post", data });

export const riderMyList = (params: { page: number; size: number; tab?: string }) =>
    request<RiderEnvelope<{ records: DeliveryOrderRow[]; total: number }>>({
        url: "o2o/deliveryOrder/myList",
        method: "get",
        params
    });

export const riderDetail = (id: number) =>
    request<RiderEnvelope<DeliveryOrderRow | null>>({ url: `o2o/deliveryOrder/detail?id=${id}`, method: "get" });

export const riderConfirmPickup = (data: { deliveryOrderId: number; photo: string[]; longitude?: number; latitude?: number }) =>
    request({ url: "o2o/deliveryOrder/confirmPickup", method: "post", data });

export const riderComplete = (data: { deliveryOrderId: number; photo: string[]; longitude?: number; latitude?: number }) =>
    request<RiderEnvelope<{ serviceFee: number }>>({ url: "o2o/deliveryOrder/complete", method: "post", data });

export const riderCancel = (data: { deliveryOrderId: number; cancelReason: string }) =>
    request({ url: "o2o/deliveryOrder/cancel", method: "post", data });

/** 移除已取消单（论坛 417866）：仅从本人列表隐藏，后台仍可查、数据不删。 */
export const riderRemoveCancelled = (data: { deliveryOrderId: number }) =>
    request({ url: "o2o/deliveryOrder/removeCancelled", method: "post", data });

export const riderLocalConfig = (shopId: number) =>
    request<any>({ url: `o2o/config/localDeliveryConfig?shopId=${shopId}`, method: "get" });

export const riderHomeConfig = (shopId: number) =>
    request<any>({ url: `o2o/config/homeServiceConfig?shopId=${shopId}`, method: "get" });

export const riderWallet = () =>
    request<RiderEnvelope<{ walletId: number; balance: number; frozenBalance: number; totalIncome: number }>>({
        url: "o2o/staffWallet/myWallet",
        method: "get"
    });

export const riderWalletLogs = (params: { page: number; size: number }) =>
    request<RiderEnvelope<{ records: any[]; total: number }>>({ url: "o2o/staffWallet/myLogs", method: "get", params });

export const riderWithdrawConfig = () => request<any>({ url: "o2o/staffWallet/withdrawConfig", method: "get" });

export const riderAccounts = () => request<RiderEnvelope<any[]>>({ url: "o2o/staffAccount/list", method: "get" });

export const riderSaveAccount = (data: {
    accountId?: number;
    accountType: number;
    accountName: string;
    accountNo: string;
    bankName?: string;
}) => request({ url: "o2o/staffAccount/save", method: "post", data });

export const riderApplyWithdraw = (data: { amount: number; accountId: number; remark?: string }) =>
    request({ url: "o2o/staffWallet/applyWithdraw", method: "post", data });

export const riderWithdrawList = (params: { page: number; size: number }) =>
    request<RiderEnvelope<{ records: any[]; total: number }>>({ url: "o2o/staffWallet/myWithdrawList", method: "get", params });

export const riderApply = (data: {
    shopId: number;
    type: number;
    name: string;
    phone: string;
    idCard?: string;
    remark?: string;
}) => request<RiderEnvelope<number>>({ url: "o2o/serviceStaff/apply", method: "post", data });

export const riderMyApplies = () =>
    request<RiderEnvelope<{ records: any[] }>>({ url: "o2o/serviceStaff/myApplies", method: "get" });

/** 买家侧派送进度（订单详情页用；非本人订单/无派送单返回 null）。 */
export const buyerDeliveryProgress = (orderId: number) =>
    request<any>({ url: `o2o/deliveryOrder/buyerProgress?orderId=${orderId}`, method: "get" });
