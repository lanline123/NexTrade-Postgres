import { request, asyncRequest } from "@/utils/request";
import type { UserFormResult,UserFormState } from "~/types/user/user";

/**
 * 获取用户详情
 * @description 获取当前登录用户的基本信息
 * @returns 用户详情数据
 */
export const getUser = () => {
    return asyncRequest<UserFormState>({
        url: "user/user/detail",
        method: "get",
        noSkipLogin: true
    });
};

/**
 * 获取会员中心数据
 * @description 获取用户中心首页展示数据（订单统计、积分等）
 * @returns 会员中心数据
 */
export const getMemberCenter = () => {
    return asyncRequest<UserFormResult>({
        url: "user/user/memberCenter",
        method: "get"
    });
};

/**
 * 获取最近浏览商品
 * @description 获取用户最近浏览过的商品列表
 * @param params - 分页参数
 * @returns 浏览历史列表
 */
export const getHistoryProduct = (params?: any) => {
    return asyncRequest<any>({
        url: "user/user/historyProduct",
        method: "get",
        params
    });
};
