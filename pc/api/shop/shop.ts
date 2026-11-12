import type { shopListResponse } from "~/types/shop/shop";

/**
 * 获取店铺装修数据
 * @description 获取店铺的装修模板数据
 * @param params - 店铺ID等参数
 * @returns 装修数据
 */
export const getShopDecorate = (params: object) => {
    return request({
        url: "shop/shop/decorate",
        method: "get",
        params
    });
};

/**
 * @description 获取商品分类列表
 * @param params 筛选条件（父级ID等）
 * @returns 分类列表
 */
export const getCategoryList = (params: object) => {
    return asyncRequest<any>({
        url: "category/category/list",
        method: "get",
        params
    });
};

/**
 * 获取店铺列表
 * @description 获取平台入驻的店铺列表
 * @param params - 筛选和分页参数
 * @returns 店铺列表
 */
export const getShopList = (params: object) => {
    return asyncRequest<shopListResponse>({
        url: "shop/shop/list",
        method: "get",
        params
    });
};
