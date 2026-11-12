import { asyncRequest } from "~/utils/request";
import type { ExchangeFilterResult } from "~/types/exchange/exchange";

/**
 * 获取积分兑换商品列表
 * @description 获取可用积分兑换的商品列表
 * @param params - 筛选和分页参数
 * @returns 积分商品列表
 */
export const getExchangeList = (params: {}) => {
    return asyncRequest<ExchangeFilterResult>({
        url: "product/exchange/list",
        method: "get",
        params
    });
};
