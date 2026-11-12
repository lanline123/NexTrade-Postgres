import { request, asyncRequest } from "@/utils/request";
import type { PaymentTypeItem, ShippingTypeItem, CheckItem, StockCalendarParams, StockCalendarResult } from "@/types/order/check";

/**
 * 获取订单结算页数据
 * @description 初始化订单确认页，返回收货地址、商品、优惠等信息
 * @param data - 结算参数（购物车ID或直接购买参数）
 * @returns 订单结算页完整数据
 */
export const getOrderCheck = (data: object) => {
    return asyncRequest<CheckItem>({
        url: "order/check/index",
        method: "post",
        data
    });
};

/**
 * 更新订单结算数据
 * @description 切换收货地址、配送方式等时重新计算订单
 * @param data - 更新的结算参数
 * @returns 更新后的订单结算数据
 */
export const updateOrderCheck = (data: object) => {
    return asyncRequest<CheckItem>({
        url: "order/check/update",
        method: "post",
        data
    });
};

/**
 * 更新订单优惠券
 * @description 选择或取消使用优惠券
 * @param data - 优惠券选择参数
 * @returns 更新后的订单结算数据
 */
export const updateCouponData = (data: any) => {
    return asyncRequest<CheckItem>({
        url: "order/check/updateCoupon",
        method: "post",
        data: data
    });
};

/**
 * 提交订单
 * @description 确认下单，生成待支付订单
 * @param data - 订单提交参数（地址、配送、发票等）
 * @returns 订单创建结果，包含订单号
 */
export const orderSubmit = (data: any) => {
    return asyncRequest<any>({
        url: "order/check/submit",
        method: "post",
        data: data
    });
};

/**
 * 获取结算页发票信息
 * @description 获取用户可选的发票抬头列表
 * @param params - 查询参数
 * @returns 发票信息列表
 */
export const getCheckInvoice = async (params: object) => {
    const result = await asyncRequest<any>({
        url: "order/check/getInvoice",
        method: "get",
        params
    });
    // 兼容返回格式：{ item }、对象或下单时保存的 JSON 字符串，统一转为对象，无数据返回 null
    let data = result?.item ?? result;
    if (typeof data === "string") {
        try {
            data = JSON.parse(data);
        } catch (error) {
            return null;
        }
    }
    return data && typeof data === "object" && Object.keys(data).length > 0 ? data : null;
};

/**
 * @description 获取可用支付方式列表
 * @param flowType 流程类型（可选）
 * @returns 支付方式列表
 */
export const getPaymentType = (flowType?: number) => {
    let url = "order/check/getAvailablePaymentType";
    if (flowType) url += "?flowType=" + flowType;
    return asyncRequest<PaymentTypeItem[]>({
        url,
        method: "get"
    });
};

/**
 * 获取店铺配送方式
 * @description 获取指定店铺的配送方式（快递、自提、同城配送等）
 * @param params - 查询参数（店铺ID等）
 * @returns 按店铺分组的配送方式列表
 */
export const getShippingType = (params: object) => {
    return asyncRequest<{ [key: string]: ShippingTypeItem[] }>({
        url: "order/check/getStoreShippingType",
        method: "get",
        params
    });
};

/**
 * 获取门店自提模板
 * @description O2O场景下获取门店的自提时间段配置
 * @param params - 查询参数（门店ID等）
 * @returns 自提时间模板数据
 */
export const getShopPickupTpl = (params: object) => {
    return asyncRequest<any>({
        url: "o2o/orderPickup/getShopPickupTpl",
        method: "get",
        params
    });
};

/**
 * 获取库存日历
 * @description O2O预约场景下获取可预约的日期和库存
 * @param params - 查询参数（商品ID、门店ID等）
 * @returns 库存日历数据
 */
export const getStockCalendar = (params: StockCalendarParams) => {
    return request<StockCalendarResult>({
        url: "o2o/OrderReservation/getStockCalendar",
        method: "get",
        params
    });
};
