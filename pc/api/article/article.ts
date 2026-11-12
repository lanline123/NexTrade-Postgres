import type { ArticleFilterParams, ArticleFilterResult, ArticleFormResult, ArticleDetailParams } from "~/types/article/article.d";
import { request } from "~/utils/request";

/**
 * 获取文章列表
 * @description 分页获取文章/资讯列表
 * @param params - 筛选和分页参数
 * @returns 文章列表
 */
export const getArticleList = (params?: ArticleFilterParams) => {
    return request<ArticleFilterResult>({
        url: "article/article/list",
        method: "get",
        params
    });
};

/**
 * 获取文章详情
 * @description 获取单篇文章的完整内容
 * @param params - 文章ID等参数
 * @param url - 操作类型（detail等）
 * @returns 文章详情
 */
export const getArticle = (params: ArticleDetailParams, url: string) => {
    return request<ArticleFormResult>({
        url: "article/article/" + url,
        method: "get",
        params
    });
};
