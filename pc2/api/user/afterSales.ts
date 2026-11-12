import { asyncRequest } from "~/utils/request";
import type { AfterSalesFilterParams, AfterSalesFilterResult, AfterSalesFormResult } from "~/types/user/afterSales";

/**
 * 获取售后列表
 * @description 获取可申请售后的订单商品列表
 * @param params - 筛选和分页参数
 * @returns 售后列表
 */
export const getAfterSalesList = (params: AfterSalesFilterParams) => {
    return asyncRequest<AfterSalesFilterResult>({
        url: "user/aftersales/list",
        method: "get",
        params
    });
};
/**
 * 获取售后配置
 * @description 获取售后服务相关配置（退货地址、规则等）
 * @returns 售后配置
 */
export const getAfterSalesConfig = (params: { orderId: number }) => {
    return asyncRequest<any>({
        url: "user/aftersales/config",
        method: "get",
        params
    });
};

/**
 * 获取售后申请数据
 * @description 获取申请售后所需的表单数据
 * @param params - 订单商品参数
 * @returns 售后申请表单数据
 */
export const getAfterSales = (params: object) => {
    return asyncRequest<AfterSalesFormResult>({
        url: "user/aftersales/applyData",
        method: "get",
        params
    });
};
/**
 * 提交售后申请
 * @description 创建售后退款/退货申请
 * @param data - 售后申请数据
 * @returns 申请结果
 */
export const updateAfterSales = (data: object) => {
    return asyncRequest({
        url: "user/aftersales/create",
        method: "post",
        data
    });
};
