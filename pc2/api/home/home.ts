import type { GrouponItemListParams, GrouponItemListResult } from "~/types/home/groupon";
import { request, asyncRequest } from "@/utils/request";
import type { HomeDataResult, CouponResponse, HomeCategoryRecommendParams, HomeCategoryRecommendData } from "@/types/home/home.d";
import type { SeckillFilterResult } from "~/types/home/seckill";
import type { FriendLinksFilterState } from "~/types/home/friendLinks";
import type { ProductFilterParams } from "~/types/product/product";
import type { SearchProductListResult } from "~/types/search/search";

/**
 * 获取首页数据
 * @description 获取PC首页完整数据（轮播图、楼层、活动等）
 * @param params - 查询参数
 * @returns 首页数据
 */
export const getHomeData = (params: {}) => {
    return request<HomeDataResult>({
        url: "home/home/pcIndex",
        method: "get",
        params
    });
};
/**
 * 获取首页推荐商品
 * @description 获取首页商品推荐列表
 * @param params - 查询参数
 * @returns 推荐商品列表
 */
export const getHomeRecommendProductData = (params: HomeCategoryRecommendParams) => {
    return asyncRequest<HomeCategoryRecommendData>({
        url: "home/home/getRecommend",
        method: "get",
        params
    });
};

/**
 * 获取首页秒杀活动
 * @description 获取当前进行中的秒杀活动商品
 * @param params - 查询参数
 * @returns 秒杀活动数据
 */
export const getHomeSeckill = (params?: object) => {
    return asyncRequest<SeckillFilterResult>({
        url: "home/home/getSeckill",
        method: "get",
        params
    });
};

/**
 * 获取首页优惠券
 * @description 获取首页展示的可领取优惠券
 * @returns 优惠券列表
 */
export const getHomeCoupon = () => {
    return asyncRequest<CouponResponse[]>({
        url: "home/home/getCoupon",
        method: "get"
    });
};

/**
 * 获取友情链接
 * @description 获取网站底部友情链接列表
 * @returns 友情链接列表
 */
export const getHomeFriendLinks = () => {
    return request<FriendLinksFilterState[]>({
        url: "home/home/friendLinks",
        method: "get"
    });
};
/**
 * 获取客服配置
 * @description 获取在线客服相关配置信息
 * @returns 客服配置
 */
export const getHomeCustomerServiceConfig = () => {
    return asyncRequest<any>({
        url: "home/home/getCustomerServiceConfig",
        method: "get"
    });
};

/**
 * 获取移动端导航
 * @description 获取移动端分类导航数据
 * @param params - 查询参数
 * @returns 导航数据
 */
export const getMobileNav = (params: any) => {
    return asyncRequest<any>({
        url: "home/home/mobileNav",
        method: "get",
        params
    });
};
/**
 * 获取商品列表
 * @description 获取推荐商品列表
 * @param params - 查询参数
 * @returns 商品列表
 */
export const getProductList = (params: ProductFilterParams) => {
    return asyncRequest<SearchProductListResult>({
        url: "home/home/getRecommend",
        method: "get",
        params
    });
};

/**
 * 获取首页拼团活动
 * @description 获取当前进行中的拼团活动列表
 * @param params - 查询参数
 * @returns 拼团活动列表
 */
export const getHomeGrouponList = (params: GrouponItemListParams) => {
    return asyncRequest<GrouponItemListResult>({
        url: "promotion/groupon/itemList",
        method: "get",
        params
    });
};

