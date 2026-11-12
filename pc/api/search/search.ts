import { request } from "~/utils/request";
import type { filterSeleted, SearchFilter, SearchFilterResult, SearchProductListResult } from "@/types/search/search.d";

/**
 * 获取搜索筛选项
 * @description 获取搜索页的筛选条件（分类、品牌、属性等）
 * @param data - 搜索参数
 * @returns 筛选项数据
 */
export const getSearchFilter = (data: {}) => {
    return request<SearchFilterResult>({
        url: "search/search/getFilter",
        method: "post",
        data: data
    });
};

/**
 * 获取ES搜索筛选建议
 * @description 基于Elasticsearch获取属性筛选建议
 * @param data - 搜索参数
 * @returns 属性建议列表
 */
export const getSearchEsFilter = (data: {}) => {
    return request<any>({
        url: "search/search/getAttributeSuggestions",
        method: "get",
        params: data
    });
};

/**
 * 获取搜索商品列表
 * @description 根据搜索条件获取商品列表
 * @param data - 搜索和筛选参数
 * @returns 商品列表
 */
export const getSearchProductList = (data: {}) => {
    return request<SearchProductListResult>({
        url: "search/search/getProduct",
        method: "post",
        data: data
    });
};
