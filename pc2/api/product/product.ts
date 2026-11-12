import type { ProductDetailParams, ProductAvailabilityParams } from "~/types/product/product";
import { request, asyncRequest } from "~/utils/request";
import type {
    ProductFilterResult,
    SkuDetail,
    ProductCouponItem,
    CommentFilterResult,
    CommentDetail,
    ProductItem,
    ProductConsultationFormState,
    ProductConsultationList,
    ProductConsultationListParams,
    PromotionList,
    ProductAmountItem,
    SkuAvailabilitys
} from "~/types/product/product.d";

/**
 * 获取商品详情
 * @description 获取商品完整信息，包括基本信息、规格、价格、库存等
 * @param params - 商品参数（商品ID等）
 * @returns 商品详情数据
 */
export const getProductDetail = (params: ProductDetailParams) => {
    return asyncRequest<ProductFilterResult>({
        // return asyncRequest<ProductFilterResult>({
        url: "product/product/detail",
        method: "get",
        params
    });
};

/**
 * IM获取商品详情
 * @description 在线客服场景下获取商品详情，支持ID或对象参数
 * @param id - 商品ID或包含ID的对象
 * @returns 商品详情数据
 */
export const getProductDetailIm = (id: number | string | ProductDetailParams) => {
    let params: ProductDetailParams = {};

    if (typeof id === "object") {
        params = id;
    } else {
        params.id = id;
    }

    return request<ProductFilterResult>({
        url: "product/product/detail",
        method: "get",
        params
    });
};

/**
 * 获取商品评价统计
 * @description 获取商品评价的统计信息（好评率、标签等）
 * @param id - 商品ID
 * @returns 评价统计数据
 */
export const getComment = (id: number) => {
    return request<CommentDetail>({
        url: "product/product/getComment",
        method: "get",
        params: { id }
    });
};

/**
 * 获取商品评价列表
 * @description 分页获取商品的用户评价
 * @param id - 商品ID
 * @param params - 分页及筛选参数
 * @returns 评价列表
 */
export const getCommentList = (id: number, params: object) => {
    return request<CommentFilterResult>({
        url: "product/product/getCommentList",
        method: "get",
        params: { id, ...params }
    });
};

/**
 * 获取商品SKU可用性详情
 * @description 根据选择的规格获取对应SKU的库存、价格等
 * @param id - 商品ID
 * @param params - 规格选择参数
 * @returns SKU详情数据
 */
export const getProductSkuDetail = (id: number, params: Omit<ProductAvailabilityParams, "id">) => {
    return request<SkuDetail>({
        // return asyncRequest<SkuDetail>({
        url: "product/product/getProductAvailability",
        method: "get",
        params: { id, ...params }
    });
};

/**
 * 添加商品到购物车
 * @description 将指定商品加入购物车
 * @param params - 添加参数（商品ID、数量、SKU等）
 * @returns 操作结果
 */
export const addToCart = (params: {
    id: number;
    number: number;
    skuId?: number;
    isQuick?: number;
    shopId?: number;
    skuItem?: Array<{ skuId: number; num: number }>;
    [key: string]: unknown;
}) => {
    return asyncRequest({
        url: "cart/cart/addToCart",
        method: "post",
        data: params
    });
};

/**
 * 积分兑换商品加入购物车
 * @description 将积分兑换商品加入购物车
 * @param params - 添加参数（商品ID、数量、SKU等）
 * @returns 操作结果
 */
export const addExchangeToCart = (params: { id: number; number: number; skuId?: number; isQuick?: number }) => {
    return asyncRequest({
        url: "product/exchange/addToCart",
        method: "post",
        data: params
    });
};

/**
 * 获取猜你喜欢列表
 * @description 基于用户行为推荐相关商品
 * @param params - 查询参数
 * @returns 推荐商品列表
 */
export const getProductLikeList = (params: {}) => {
    return asyncRequest<ProductItem[]>({
        url: "common/recommend/guessLike",
        method: "get",
        params
    });
};

/**
 * 获取推荐商品ID列表
 * @description 获取猜你喜欢的商品ID集合，用于懒加载
 * @returns 商品ID列表
 */
export const getProductLikeListIds = () => {
    return asyncRequest<any>({
        url: "common/recommend/getProductIds",
        method: "get"
    });
};

/**
 * 获取商品咨询列表
 * @description 获取商品的用户咨询问答列表
 * @param params - 分页参数
 * @returns 咨询列表
 */
export const getProductConsultationList = (params: ProductConsultationListParams) => {
    return request<ProductConsultationList>({
        url: "product/product/getFeedbackList",
        method: "get",
        params
    });
};
/**
 * 提交商品咨询
 * @description 用户对商品提出咨询问题
 * @param data - 咨询内容
 * @returns 提交结果
 */
export const addConsultation = (data: ProductConsultationFormState) => {
    return request({
        url: "user/feedback/submit",
        method: "post",
        data
    });
};

/**
 * 获取售后服务内容
 * @description 获取平台配置的售后服务说明
 * @param id - 商品ID（预留参数）
 * @returns 售后服务内容
 */
export const getAfterSaleService = (id: number) => {
    return request<any>({
        url: "common/config/afterSalesService",
        method: "get"
    });
};

/**
 * 检查商品收藏状态
 * @description 判断当前用户是否已收藏该商品
 * @param params - 商品参数
 * @returns 收藏状态
 */
export const getCollectProduct = (params: object) => {
    return request<any>({
        url: "product/product/isCollect",
        method: "get",
        params
    });
};

/**
 * 获取商品可用优惠券
 * @description 获取当前商品可领取的优惠券列表
 * @param params - 商品参数
 * @returns 优惠券列表
 */
export const getProductCouponList = (params: object) => {
    return asyncRequest<ProductCouponItem[]>({
        url: "product/product/getCoupon",
        method: "get",
        params
    });
};

/**
 * 获取店铺详情
 * @description 获取店铺完整信息
 * @param shopId - 店铺ID
 * @returns 店铺详情数据
 */
export const getShopDetail = (shopId: number) => {
    return asyncRequest<any>({
        url: "shop/shop/detail",
        method: "get",
        params: { shopId }
    });
};

/**
 * 获取店铺头部信息
 * @description 获取店铺简要信息（用于页面头部展示）
 * @param shopId - 店铺ID
 * @returns 店铺头部数据
 */
export const getShopHead = (shopId: number) => {
    return asyncRequest<any>({
        url: "shop/shop/head",
        method: "get",
        params: { shopId }
    });
};
/**
 * 获取店铺分类树
 * @description 获取店铺的商品分类结构
 * @param shopId - 店铺ID
 * @returns 分类树数据
 */
export const getShopTree = (shopId: number) => {
    return asyncRequest<any>({
        url: "shop/category/tree",
        method: "get",
        params: { shopId }
    });
};

/**
 * 收藏/取消收藏店铺
 * @description 切换店铺收藏状态
 * @param shopId - 店铺ID
 * @returns 操作结果
 */
export const getShopCollect = (shopId: number | any) => {
    return asyncRequest<any>({
        url: "shop/shop/collect",
        method: "post",
        data: { shopId }
    });
};

/**
 * 获取商品促销活动信息
 * @description 获取商品参与的促销活动（满减、满赠等）
 * @param data - 商品参数
 * @returns 按类型分组的促销活动列表
 */
export const getPromotion = (data: object) => {
    // 促销活动为商品列表加载完成后的补充请求，均在 await 之后触发；SSR 下此时已脱离
    // Nuxt 上下文，useFetch 会抛 "nuxt instance unavailable"，故改用 $fetch（asyncRequest）。
    return asyncRequest<{ [key: string]: PromotionList }>({
        url: "product/product/promotion",
        method: "post",
        data
    });
};

/**
 * 获取积分兑换商品详情
 * @description 获取积分商城商品的详细信息
 * @param id - 商品ID
 * @returns 积分商品详情
 */
export const getExchangeDetail = (id: any) => {
    return request<ProductFilterResult>({
        url: "product/exchange/detail",
        method: "get",
        params: { id }
    });
};

/**
 * 获取商品金额计算
 * @description 根据数量和规格计算商品总价
 * @param data - 计算参数
 * @returns 金额计算结果
 */
export const getProductAmount = (data: object) => {
    return request<ProductAmountItem>({
        url: "product/product/getProductAmount",
        method: "post",
        data
    });
};

/**
 * 商品询价
 * @description 提交商品询价请求
 * @param data - 询价参数
 * @returns 询价结果
 */
export const enquiry = (data: object) => {
    return asyncRequest({
        url: "product/product/priceInquiry",
        method: "post",
        data
    });
};

/**
 * 批量获取商品SKU可用性
 * @description 批量查询多个商品的SKU库存状态
 * @param params - 商品ID列表等参数
 * @returns 按商品分组的SKU可用性数据
 */
export const getBatchProductAvailability = (params: object) => {
    return asyncRequest<{ [key: string]: SkuAvailabilitys }>({
        url: "product/product/getBatchProductAvailability",
        method: "get",
        params
    });
};
