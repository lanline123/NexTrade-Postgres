import request from "@/utils/request";
import { isO2o } from "@/utils";
import { normalizeProductQuery } from "@/utils/productQuery";
import type { ExchangeListResponse } from "@/types/exchange/exchange";
import type { AddCartResponse } from "@/types/product/product";

/**
 * @description 获取积分兑换商品列表
 * @param params 筛选条件
 * @returns 兑换商品列表
 */
export const getExchangeList = (params: object) => {
    return request<ExchangeListResponse>({
        url: "product/exchange/list",
        method: "get",
        params
    });
};

/**
 * @description 获取积分兑换商品详情
 * @param params 包含商品ID
 * @returns 商品详情
 */
export const getExchangeDetail = (params: object) => {
    return request({
        url: "product/exchange/detail",
        method: "get",
        params: normalizeProductQuery(params, isO2o())
    });
};

/**
 * @description 积分兑换商品加入购物车
 * @param params 商品ID、数量等
 * @returns 加入结果
 */
export const addExchangeToCart = (params: AnyObject) => {
    return request<AddCartResponse>({
        url: "product/exchange/addToCart",
        method: "post",
        data: params
    });
};

/**
 * @description 积分兑换优惠券（我方新增：即时扣积分发券，不走购物车）
 * @param data 兑换品ID
 * @returns 兑换结果（已发券ID、本次扣除积分、剩余积分、积分价来源）
 */
export const exchangeCoupon = (data: { id: number }) => {
    return request({
        url: "product/exchange/exchangeCoupon",
        method: "post",
        data
    });
};
