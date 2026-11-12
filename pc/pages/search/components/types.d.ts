/**
 * 搜索页（PC）自包含类型定义
 * 仅供 pages/search 下的组件使用，独立于全局 types/search，避免与旧搜索页类型冲突。
 */

export type ID = string | number;
export type MoneyLike = string | number;

/** 属性筛选项（PC / DTO 双形态） */
export interface SearchAttrObj {
    attrId?: string;
    attrValue?: string;
}

/** 店铺精简信息 */
export interface ApiSearchShopVO {
    shopId?: ID;
    shopTitle?: string;
    shopLogo?: string;
}

/** 商品 SKU 精简信息 */
export interface ApiSearchProductSkuVO {
    id?: ID;
    productId?: ID;
    skuValue?: string;
    skuSn?: string;
    skuStock?: number;
    skuPrice?: MoneyLike;
    marketPrice?: MoneyLike;
}

/** 搜索商品列表项 */
export interface ApiSearchProductListResVO {
    id?: ID;
    productId?: ID;
    categoryId?: ID;
    brandId?: ID;
    productTsn?: string;
    marketPrice?: MoneyLike;
    shippingTplId?: ID;
    freeShipping?: number;
    picUrl?: string;
    picLarge?: string;
    picThumb?: string;
    productBrief?: string;
    productName?: string;
    checkStatus?: number;
    shopId?: ID;
    suppliersId?: ID;
    productType?: number;
    saleMode?: number;
    productSaleMode?: number;
    productSn?: string;
    productPrice?: MoneyLike;
    productStatus?: number;
    isBest?: number;
    isNew?: number;
    isHot?: number;
    productStock?: number;
    sortOrder?: number;
    productSku?: ApiSearchProductSkuVO[];
    shop?: ApiSearchShopVO;
    isTimeDiscount?: boolean;
    price?: MoneyLike;
    firstWholesalePrice?: MoneyLike | null;
    wholesaleCardMoq?: number | null;
    wholesaleMultiple?: number | null;
    orgProductPrice?: string;
    isSeckill?: boolean;
    isGroupon?: boolean;
    limitNumber?: number;
    virtualSales?: number;
    /** 2 表示周期购 / 次卡 */
    deliveryMode?: number;
    /** 1 周期购，2 次卡兑换 */
    deliveryType?: number;
}

/** 列表行：后端 VO + 前端 UI 字段 */
export type SearchProductItem = ApiSearchProductListResVO & {
    number?: number;
    seoLink?: string;
    seckillStock?: number;
};

/** 分类项 */
export interface ApiSearchCategoryVO {
    id?: ID;
    categoryName?: string;
    parentId?: ID;
    sortOrder?: number;
    isShow?: number;
    categoryPic?: string;
    categoryIco?: string;
    isHot?: number;
    /** 分类 SEO 链接；后端未下发时由前端回退基础链接 */
    seoLink?: string;
}

/** 品牌项 */
export interface ApiSearchBrandVO {
    id?: ID;
    brandName?: string;
    brandLogo?: string;
}

/** PC 筛选面板：在品牌上扩展首字母 */
export type SearchFilterBrandRow = ApiSearchBrandVO & { firstWord?: string };

/** 店铺分类项 */
export interface ApiSearchShopProductCategoryVO {
    id?: ID;
    categoryName?: string;
    shopId?: ID;
    parentId?: ID;
    sortOrder?: number;
    isShow?: number;
    categoryPic?: string;
}

/** PC 筛选面板聚合 */
export interface SearchFilterListResult {
    brand?: SearchFilterBrandRow[];
    category?: ApiSearchCategoryVO[];
    shopCategory?: ApiSearchShopProductCategoryVO[];
    maxPrice?: MoneyLike;
    attrs?: SearchAttrObj[];
}

/** PC 搜索页：筛选表单状态 */
export interface SearchFilter {
    page: number;
    categoryId: string;
    pId: string;
    brandIds: string[];
    attrs: SearchAttrObj[];
    sortField: string;
    sortOrder: string;
    couponId: string;
    minPrice: number;
    maxPrice: number;
    keyword: string;
}

/** 请求查询参数 */
export interface QueryParams {
    cat?: string;
    brand?: string;
    sort?: string;
    order?: string;
    page?: number;
    max?: number;
    min?: number;
    keyword?: string;
    pId?: string;
    couponId?: string;
    shopId?: string | number;
    [key: string]: string | number | string[] | undefined;
}

/** 已选择筛选项（兼容树形字段） */
export interface filterSeleted {
    id?: string;
    intro?: string;
    keyword?: string;
    keywords?: string;
    category?: string;
    brand?: string;
    shopCategory?: string;
    categoryId?: string;
    categoryName?: string;
    categoryDesc?: string;
    nparentId?: string;
    sortOrder?: number;
    measureUnit?: string;
    isShow?: number;
    seoTitle?: string;
    shortName?: string;
    categoryPic?: string;
    categoryIco?: number;
    isHot?: number;
    searchKeywords?: string;
    children?: filterSeleted[];
    attrs?: SearchAttrObj[];
}
