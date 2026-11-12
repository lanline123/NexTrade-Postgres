export interface shopListResponse {
    records: shopListItem[];
    total: number;
}

export interface shopListItem {
    statusText: string;
    shopId: number;
    shopTitle: string;
    seoLink?: string;
    addTime: string;
    shopLogo: string;
    /** 店铺评分（可能不存在） */
    shopStar?: number;
    clickCount: number;
    status: number;
    merchantId: number;
    shopMoney: string;
    frozenMoney: string;
    contactMobile: string;
    description: string;
    kefuPhone: string;
    kefuWeixin: string;
    kefuLink: string;
    isContactKefu: number;
    kefuInlet: any[] | null;
    collectCount: number;
    /** 已通过合规快照的营业执照图；空串或 null 表示前台不公示 */
    businessLicensePic?: string | null;
    listingCount: number;
    listingProduct: shopListListingProduct[];
}

export interface shopListListingProduct {
    productId: number;
    productName: string;
    productSn: string;
    seoLink?: string;
    productTsn: string;
    productStock: number;
    productPrice: string;
    marketPrice: string;
    shippingTplId: number;
    productStatus: number;
    productType: number;
    categoryId: number;
    brandId: number;
    shopId: number;
    keywords: string;
    shopCategoryId: number;
    checkStatus: number;
    checkReason: string;
    clickCount: number;
    productWeight: string;
    isPromote: number;
    isPromoteActivity: number;
    promotePrice: string;
    promoteStartDate: string;
    promoteEndDate: string;
    seckillMaxNum: number;
    productBrief: string;
    productDesc: string;
    picUrl: string;
    picThumb: string;
    picOriginal: string;
    commentTag: null;
    freeShipping: number;
    integral: number;
    addTime: number;
    sortOrder: number;
    storeSortOrder: number;
    isDelete: number;
    isBest: number;
    isNew: number;
    isHot: number;
    lastUpdate: number;
    remark: string;
    giveIntegral: number;
    rankIntegral: number;
    suppliersId: number;
    virtualSales: number;
    limitNumber: number;
    productCare: string;
    productRelated: any[];
    productServiceIds: number[];
    isSupportReturn: number;
    isSupportCod: number;
    productVideo: string;
    prepayPrice: string;
}
