import { request } from "~/utils/request";
import type { CategoryFilterState } from "~/types/article/category.d";

export const getBzzxCategoryList = (params: object) => {
    return request<CategoryFilterState[]>({
        url: "article/category/indexBzzxList",
        method: "get",
        params
    });
};
