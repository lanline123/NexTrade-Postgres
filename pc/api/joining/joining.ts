import { request, asyncRequest } from "~/utils/request";
import type { CommentFilterResult, JoiningDetail, JoiningActivityDetail, GrouponPromotionDetail, GrouponPromotionImageResult } from "~/types/joining/joining";

// 获取拼团商品列表
export const getProductList = (params: object) => {
    return asyncRequest<CommentFilterResult>({
        url: "promotion/groupon/itemList",
        method: "get",
        params
    });
};

//获取拼团详情
export const getJoiningDetail = (params: object) => {
    return asyncRequest<JoiningDetail>({
        url: "promotion/groupon/record/details",
        method: "get",
        params
    });
};

// 获取拼团记录列表
export const getProductRecordList = (params: object) => {
    return asyncRequest({
        url: "promotion/groupon/record/list",
        method: "get",
        params
    });
};

//获取拼团活动详情
export const getJoiningActivityDetail = (params: object) => {
    return request<JoiningActivityDetail>({
        url: "promotion/groupon/getGrouponInfo",
        method: "get",
        params
    });
};

// 获取拼团活动详情
export const getGrouponPromotionDetail = (params: object) => {
    return asyncRequest<GrouponPromotionDetail>({
        url: "promotion/groupon/detail",
        method: "get",
        params
    });
};

// 获取进行中的拼团活动图标列表（PC 图等）
export const getGrouponPromotionImageList = (params: object = {}) => {
    return asyncRequest<GrouponPromotionImageResult>({
        url: "promotion/groupon/imageList",
        method: "get",
        params
    });
};
