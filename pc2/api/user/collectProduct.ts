import { asyncRequest } from "~/utils/request";
import type { CollectProductFilterParams, CollectProductFilterResult } from "~/types/user/collectProduct.d";

/**
 * 获取收藏商品列表
 * @description 获取用户收藏的商品列表
 * @param params - 分页参数
 * @returns 收藏商品列表
 */
export const getCollectProductList = (params?: CollectProductFilterParams) => {
    return request<CollectProductFilterResult>({
        url: "user/collectProduct/list",
        method: "get",
        params
    });
};

/**
 * 取消收藏商品
 * @description 从收藏列表中移除商品
 * @param data - 商品ID
 * @returns 操作结果
 */
export const delCollectProduct = (data: object) => {
    return asyncRequest({
        url: "user/collectProduct/cancel",
        method: "post",
        data
    });
};

/**
 * 收藏商品
 * @description 将商品加入收藏列表
 * @param data - 商品ID
 * @returns 操作结果
 */
export const updateCollectProduct = (data: object) => {
    return asyncRequest({
        url: "user/collectProduct/save",
        method: "post",
        data
    });
};
