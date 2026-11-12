import type { PointFilterParams, PointFilterResult } from "~/types/user/point.d";
// 获取商品分类列表

export const getPointList = (params: PointFilterParams) => {
    return asyncRequest<PointFilterResult>({
        url: "user/pointsLog/list",
        method: "get",
        params
    });
};
