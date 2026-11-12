import { request, asyncRequest } from "~/utils/request";
import type { CategoryNavigation } from "@/types/product/category.d";
import type { ProductConsultationItem } from "~/types/product/product";

/**
 * 获取父级分类树
 * @description 获取指定分类的所有父级分类（面包屑导航用）
 * @param id - 分类ID
 * @returns 父级分类树
 */
export const getParentCategoryTree = (id: number) => {
    return request<CategoryNavigation[]>({
        url: "category/category/parentTree",
        method: "get",
        params: { id }
    });
};

/**
 * 获取热门分类列表
 * @description 获取首页装修分类导航使用的热门分类
 * @returns 热门分类列表
 */
export const getCategoryHot = () => {
    return asyncRequest<any>({
        url: "category/category/hot",
        method: "get"
    });
};

/**
 * 获取相关分类
 * @description 获取与当前分类相关的其他分类
 * @param params - 查询参数
 * @returns 相关分类列表
 */
export const getProductRelateCategory = (params: object) => {
    return asyncRequest<ProductConsultationItem[]>({
        url: "category/category/relateCategory",
        method: "get",
        params
    });
};

/**
 * 获取相关品牌
 * @description 获取分类下的关联品牌
 * @param params - 查询参数
 * @returns 相关品牌列表
 */
export const getProductRelateBrand = (params: object) => {
    return asyncRequest<ProductConsultationItem[]>({
        url: "category/category/relateBrand",
        method: "get",
        params
    });
};
/**
 * 获取相关排行
 * @description 获取分类下的商品销量排行
 * @param params - 查询参数
 * @returns 排行榜数据
 */
export const getProductRelateRank = (params: object) => {
    return asyncRequest<any>({
        url: "category/category/relateRank",
        method: "get",
        params
    });
};

/**
 * 获取相关文章
 * @description 获取与分类相关的资讯文章
 * @param params - 查询参数
 * @returns 相关文章列表
 */
export const getProductRelateArticle = (params: object) => {
    return asyncRequest<any>({
        url: "category/category/relateArticle",
        method: "get",
        params
    });
};
/**
 * 获取看了又看推荐
 * @description 获取浏览过该商品的用户还看了哪些商品
 * @param params - 查询参数
 * @returns 推荐商品列表
 */
export const getProductRelateLookAlso = (params: object) => {
    return asyncRequest<any>({
        url: "category/category/relateLookAlso",
        method: "get",
        params
    });
};

/**
 * 获取关联商品列表
 * @description 获取商品的关联推荐商品
 * @param params - 查询参数
 * @returns 关联商品列表
 */
export const getProductRelateProductList = (params: object) => {
    return asyncRequest<any>({
        url: "product/product/getRelated",
        method: "get",
        params
    });
};
