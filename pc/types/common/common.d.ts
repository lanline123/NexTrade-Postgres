export interface LangListResponse {
    data: LangListItem[];
    errcode: number;
    message: string;
}

export interface LangListItem {
    id: number;
    localeCode: string;
    language: string;
    flagPicture: string;
    lastUpdated: string;
    isEnabled: number;
    isDefault: number;
    /** 该语言的独立域名，不含协议与端口；未配置时为空串 */
    domain?: string;
}

export interface DefaultLangResponse {
    item: DefaultLangItem;
    errcode: number;
    message: string;
}
export interface DefaultLangItem {
    currencyId: number;
    flagPicture: string;
    id: number;
    isDefault: number;
    isEnabled: number;
    language: string;
    lastUpdated: string;
    localeCode: string;
    sort: number;
    /** 该语言的独立域名，不含协议与端口；未配置时为空串 */
    domain?: string;
}

export interface CurrencyResponse {
    item: CurrencyItem[];
    errcode: number;
    message: string;
}

export interface CurrencyItem {
    id: number;
    isDefault: number;
    name: string;
    rate: string;
    symbol: string;
}

export interface MobileAreaCodeResponse {
    list: MobileAreaCodeList[];
    errcode: number;
    message: string;
}

export interface MobileAreaCodeList {
    code: string;
    id?: number;
    isAvailable?: number;
    isDefault?: number;
    label?: string;
    name?: string;
}

export interface ProductsListResponse {
    records: ProductsListResult[];
    filter: Filter;
    total: number;
    waitingCheckedCount: number;
    errcode: number;
    message: string;
}

export interface Filter {
    isShow: number;
    page: number;
    size: number;
    sortField: string;
    sortOrder: string;
    productId: number;
    isDelete: number;
    categoryId: number;
    brandId: number;
    ids: null;
    shopId: number;
    introType: string;
    couponId: number;
    shopCategoryId: number;
    withCartSum: number;
    productStatus: number;
}

export interface ProductsListResult {
    productBrief?: string;
    productSaleMode?: number;
    saleMode?: number;
    isSeckill?: number;
    seckillStock?: number;
    virtualSales?: number;
    price?: string;
    categoryId: number;
    brandId: number;
    productTsn: string;
    marketPrice: string;
    shippingTplId: number;
    freeShipping: number;
    productId: number;
    seoLink?: string;
    picUrl: string;
    picThumb: string;
    productName: string;
    checkStatus: number;
    checkReason: string;
    shopId?: number;
    suppliersId: number;
    productType: number;
    productSn: string;
    productPrice: string;
    productStatus: number;
    isBest: number;
    isNew: number;
    isHot: number;
    productStock: number;
    sortOrder: number;
    productSku: ProductSku[];
    shop: ShopSimple | null;
    deliveryMode?: number; // 交付模式：0-单次交付，2-多次交付（周期购）
    deliveryType?: number; // 交付类型：1-按计划配送，2-按需兑换（次卡）
}

export interface ProductSku {
    skuId: number;
    productId: number;
    skuValue: string;
    skuData: SkuDatum[];
    skuSn: string;
    skuStock: number;
    skuTsn: string;
    skuPrice: string;
}

export interface SkuDatum {
    name: string;
    value: string;
}

export interface ShopSimple {
    statusText: string;
    shopId: number;
    shopTitle: string;
}

export interface BaseConfigResponse {
    shopName: string;
    shopTitle: string;
    shopTitleSuffix: string;
    shopLogo: string;
    shopKeywords: string;
    shopDesc: string;
    storageUrl: string;
    dollarSign: string;
    dollarSignCn: string;
    icoImg: string;
    autoRedirect: number;
    openWechatOauth: number;
    personApplyEnabled: number;
    h5Domain: string;
    pcDomain: string;
    adminDomain: string;
    showService: number;
    versionType: string;
    shopIcpNo: string;
    shopIcpNoUrl: string;
    businessLicense: string;
    ediLicense: string;
    businessLicenseType: string[];
    /** 经营许可证展示开关：1 开启，0 关闭；缺失或非法值由前端按开启处理 */
    businessLicenseEnabled?: number;
    shop110No: string;
    shop110Link: string;
    shopCompanyTxt: string;
    companyAddress: string;
    kefuPhone: string;
    kefuTime: string;
    kefuAddress: string;
    isEnterprise: number;
    deCopyright: number;
    poweredByStatus: number;
    poweredBy: number;
    categoryDecorateType: number;
    canInvoice: number;
    invoiceAdded: number;
    defaultShopName: string;
    defaultLogisticsName: string;
    isOpenMobileAreaCode: number;
    showSelledCount: number;
    showMarketprice: number;
    useSurplus: number;
    usePoints: number;
    useCoupon: number;
    memberWithdrawalEnabled?: number; // 会员余额提现开关，0-关闭，1-开通（D1107）
    memberWithdrawalReceiptMethod?: number[]; // 提现到账方式 1银行卡 2支付宝 3微信（D1107）
    memberWithdrawalDescription?: string; // 会员余额提现说明（HTML，D1197）
    closeOrder: number;
    shopRegClosed: number;
    companyDataType: number;
    companyDataTips: string;
    isIdentity: number;
    isEnquiry: number;
    growUpSetting: GrowUpSetting;
    decoratePageConfig: DecoratePageConfig;
    defaultHeaderStyle: number;
    errcode: number;
    message: string;
    openWechatPcLogin: number;
    wechatRegisterBindPhone: number;
    googleLoginOn: number;
    facebookLoginOn: number;
    twitterLoginOn: number;
    defaultTechSupport: string;
    poweredByLogo: string;
    openEmailRegister: number;
    integralName: string;
    closeAuth: number;
    bulkPurchase: number;
    enableAttributeFilter: number;
    /** 是否开启商品评价：1开启，0关闭（D1146）。可选：老后端不下发，前端按开启兜底 */
    productRawImageDownloadEnabled?: number;
    productRawImageDownloadChannels?: string[];
    productRawImageDownloadScopes?: string[];
    productCommentEnabled?: number;
    /** 是否开启匿名评价：1开启，0关闭（D1146）。仅在 productCommentEnabled=1 时有意义 */
    anonymousCommentEnabled?: number;
    defaultAvatar: string;
    /** 是否启用搜本店：1开启，0关闭 */
    searchSelfStore: number;
    /** 首页是否开启“搜门店”：1开启，0关闭 */
    indexSearchShop: number;
    /** 当前生效的地图服务商：amap / tianditu；缺失或未知值按 amap 处理 */
    mapProvider?: string;
    /** 天地图 JS API 的 tk，仅在 mapProvider=tianditu 时需要 */
    tiandituJsTk?: string;
    /** 高德 JS API Key */
    amapJsKey?: string;
    /** 高德 JS API 安全密钥 */
    amapJsSecret?: string;
}

export interface ThemeSettingsResponse {
    themeStyle: string;
}

export interface DecoratePageConfig {
    type: string;
    module: any[];
    backgroundRepeat: string;
    backgroundSize: string;
    style: number;
    title: string;
    titleColor: string;
    headerStyle: number;
    titleBackgroundColor: string;
    backgroundImage: BackgroundImage;
    backgroundColor: string;
}

export interface BackgroundImage {
    picUrl: string;
    picThumb: string;
}

export interface GrowUpSetting {
    buyOrder: number;
    buyOrderNumber: number;
    buyOrderGrowth: string;
    evpi: number;
    evpiGrowth: string;
    bindPhone: number;
    bindPhoneGrowth: string;
}

export interface ThemeStyle {
    themeId: number;
    "--general": string;
    "--main-bg": string;
    "--main-bg-gradient": string;
    "--main-text": string;
    "--vice-bg": string;
    "--vice-text": string;
    "--icon": string;
    "--price": string;
    "--tag-text": string;
    "--tag-bg": string;
    "--primary-light-3": string;
    "--primary-light-5": string;
    "--primary-light-7": string;
    "--primary-light-8": string;
    "--primary-light-9": string;
    "--primary-dark-2": string;
    "--main-btn-hover-bg": string;
    "--ump-main-bg": string;
    "--ump-main-text": string;
    "--ump-vice-bg": string;
    "--ump-vice-text": string;
    "--ump-icon": string;
    "--ump-price": string;
    "--ump-tag-text": string;
    "--ump-tag-bg": string;
    "--ump-coupon-bg": string;
    "--ump-border": string;
    "--ump-start-bg": string;
    "--ump-end-bg": string;
}
