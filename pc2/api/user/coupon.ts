import type { CouponFilterParams, CouponFilterResult,MyCouponFilterState } from "~/types/user/coupon.d";

/**
 * 获取可领优惠券列表
 * @description 获取当前可领取的优惠券列表
 * @param params - 筛选参数
 * @returns 优惠券列表
 */
export const getCouponList = (params: CouponFilterParams) => {
    return asyncRequest<CouponFilterResult>({
        url: "user/coupon/getList",
        method: "get",
        params
    });
};
/**
 * 获取我的优惠券列表
 * @description 获取用户已领取的优惠券
 * @param params - 筛选参数（状态等）
 * @returns 我的优惠券列表
 */
export const getMyCouponList = (params: CouponFilterParams) => {
    return asyncRequest<CouponFilterResult>({
        url: "user/coupon/list",
        method: "get",
        params
    });
};

/**
 * 领取优惠券
 * @description 用户领取指定优惠券
 * @param data - 优惠券ID
 * @returns 领取结果
 */
export const addCoupon = (data: object) => {
    return asyncRequest({
        url: "user/coupon/claim",
        method: "post",
        data
    });
};
/**
 * 删除优惠券
 * @description 删除用户的优惠券（已过期等）
 * @param data - 优惠券ID
 * @returns 操作结果
 */
export const delCoupon = (data: object) => {
    return asyncRequest({
        url: "user/coupon/del",
        method: "post",
        data
    });
};

/**
 * 获取优惠券详情
 * @description 获取指定优惠券的详细信息
 * @param id - 优惠券ID
 * @returns 优惠券详情
 */
export const getMyCouponInfo = (id: any) => {
    return asyncRequest<any>({
        url: "user/coupon/detail",
        method: "get",
        params: { id }
    });
};
