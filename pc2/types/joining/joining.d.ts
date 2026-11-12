export interface CommentFilterResult {
    records: JoiningItem[];
    total: number;
}

export interface JoiningItem {
    productId: number;
    productSn?: string;
    seoLink?: string;
    productName: string;
    minGrouponPrice: number;
    productPrice: number;
    grouponPromotionItemId: number;
    // 成团人数配置（后端通常返回 JSON 字符串，如：[{ groupNum: 2, ... }, ...]）
    groupNum?: string;
    picThumb: string;
    picUrl: string;
    joinNumCount: number;
    endTime?: string;
    countdownText?: string;
    // 二期会员标签（D1096）：false 时拼团正常展示但置灰参与入口并提示
    canJoin?: boolean;
    restrictTip?: string;
}

export interface SearchFilter {
    page: number;
    categoryId: number;
    brandIds: number[];
    sortField: string;
    sortOrder: string;
    couponId: number;
    minPrice: number;
    maxPrice: number;
    keyword: string;
}

// 列表查询时筛选参数类型
export interface OrderFilterParams {
    page: number;
    size?: number;
    sortField?: string;
    sortOrder?: string;
    keyword?: string;
    payStatus?: number;
    orderStatus?: number;
    addTime?: string[];
    shippingStatus?: number;
    commentStatus?: number;
    dateType?: number;
    joinType?: number;
    isMine: boolean;
    joining?: number;
}

// 获取列表返回参数类型
export interface OrderFilterState {
    orderAddTime: string;
    grouponPromotionRecordId: number;
    grouponPromotionItemId: number;
    productName: string;
    seoLink?: string;
    picThumb: string;
    quantity: number;
    minGrouponPrice: number;
    joinStatus: number;
    productSn: number;
    payTypeId: number;
    grouponPrice: number;
    orderId: number;
    orderSn: string;
}

export interface QueryParams {
    cat: number;
    brand: string;
    sort: string;
    order: string;
    page: number;
    max: number;
    min: number;
    keyword: string;
    [key: string]: string | number | string[] | undefined;
}

export interface JoiningDetail {
    grouponPromotionRecordId: number;
    grouponPromotionItemId: number;
    productId?: number;
    productName: string;
    seoLink?: string;
    joinType: number;
    joinStatus: number;
    minGroupNum: number;
    shopId: number;
    shopTitle: string;
    grouponImage: string;
    minGrouponPrice: number;
    productPrice: number;
    picThumb: string;
    joinNumCount: number;
    joinUsers: JoinUser[];
    endTime: string;
    orderId: number;
    productSn: string;
    joinStatusDesc: string;
}

export interface JoinUser {
    userId: number;
    username: string;
    avatar: string;
    leader?: boolean;
    classIcon?: string;
}

export interface JoiningActivityDetail {
    grouponPromotionItemId: number;
    grouponPromotionId: number;
    productId: number;
    productSn: string;
    seoLink?: string;
    endTime: string;
}

export interface GrouponPromotionDetail {
    pcGrouponImage?: string;
    [key: string]: any;
}

// promotion/groupon/image-list 返回项
export interface GrouponPromotionImageItem {
    grouponPromotionId: number;
    name: string;
    groupType: number;
    grouponImage: string;
    pcGrouponImage: string;
    shareImage: string;
    shopId: number;
    addTime: string;
}

export interface GrouponPromotionImageResult {
    records: GrouponPromotionImageItem[];
    total: number;
}
