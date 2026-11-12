/**
 * 搜索页（PC）API 适配层
 * 将页面组件使用的命名空间调用（searchApi / shopApi / decorateApi）
 * 映射到 o2o 现有的接口函数，返回值使用宽松类型以避免跨类型体系摩擦。
 */
import { request } from "~/utils/request";
import { getSearchFilter, getSearchEsFilter, getSearchProductList } from "~/api/search/search";
import { getImageSearchAttributes, getImageSearchFilter, getImageSearchProductList, isImageSearchExpired } from "~/api/search/imageSearch";
import { getShopList } from "~/api/shop/shop";

/** 带图搜编号的请求只在本次图片召回的商品内查询，需要游客标识并保留接口返回的失败原因 */
const isImageSearch = (data: Record<string, unknown>): boolean => Boolean(data?.searchToken);

/** 图搜编号无效或已过期按没有数据处理，不提示失效也不重新检索 */
const emptyWhenExpired =
    <T>(empty: T) =>
    (error: unknown): T => {
        if (isImageSearchExpired(error)) return empty;
        throw error;
    };

export const searchApi = {
    getFilter: (data: Record<string, unknown>) =>
        (isImageSearch(data) ? getImageSearchFilter(data).catch(emptyWhenExpired({ filter: {}, filterSelected: {} })) : getSearchFilter(data)) as Promise<any>,
    getEsFilter: (data: Record<string, unknown>) =>
        (isImageSearch(data) ? getImageSearchAttributes(data).catch(emptyWhenExpired({ attributes: [] })) : getSearchEsFilter(data)) as Promise<any>,
    getProductList: (data: Record<string, unknown>) =>
        (isImageSearch(data) ? getImageSearchProductList(data).catch(emptyWhenExpired({ records: [], total: 0 })) : getSearchProductList(data)) as Promise<any>
};

export const shopApi = {
    getList: (params: Record<string, unknown>) => getShopList(params) as Promise<any>
};

export const decorateApi = {
    /** PC 搜索页装修；传入 previewId 时拉预览草稿 */
    getPcSearch: (params?: Record<string, unknown>) => request<any>({ url: "home/home/pcSearch", method: "get", params })
};
