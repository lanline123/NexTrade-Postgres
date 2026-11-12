import { asyncRequest } from "~/utils/request";
import type { CollectShopFilterParams, CollectShopFilterResult } from "~/types/user/collectShop.d";

// 收藏商店铺列表
export const getCollectShopList = (params?: CollectShopFilterParams) => {
    return request<CollectShopFilterResult>({
        url: "user/user/collectionShop",
        method: "get",
        params
    });
};

// 取消收藏商店铺
export const delCollectShop = (data: object) => {
    return asyncRequest({
        url: "user/user/collectionShop/cancel",
        method: "post",
        data
    });
};
// 收藏店铺
export const updateCollectShop = (data: object) => {
    return asyncRequest({
        url: "user/user/collectionShop/save",
        method: "post",
        data
    });
};
