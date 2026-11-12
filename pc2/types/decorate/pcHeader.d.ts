/**
 * PC 头部装修配置（前台运行时）
 * 字段以 NexTrade-Admin 编辑器存盘为权威（pageModule.pcHeader），与 Admin pc-header/types.ts 对齐。
 */

export interface PcHeaderPic {
    picUrl?: string;
    picThumb?: string;
}

export interface PcHeaderLink {
    link?: string;
    name?: string;
    [key: string]: unknown;
}

export interface PcHeaderNoticeItem {
    picId?: number | string;
    picThumb?: string;
    picUrl?: string;
    picName?: string;
    picTitle?: string;
    picDesc?: string;
    picLink?: PcHeaderLink;
}

export interface PcHeaderNoticeGroup {
    notices?: PcHeaderNoticeItem[];
}

export interface PcHeaderMemberCenterItem {
    picTitle?: string;
    picDesc?: string;
    picLink?: PcHeaderLink;
    /** 兼容旧版会员中心菜单字段 */
    title?: string;
    link?: string;
}

export interface PcHeaderConfig {
    /** 头部风格：1=简约风，2=跨境风 */
    styleType?: 1 | 2;
    isWhite?: 0 | 1;

    // 主导航
    showMainNav?: 0 | 1;
    mainNavPaddingTop?: number;
    mainNavPaddingBottom?: number;

    // 顶部菜单栏（开关项为简约风专属，两个颜色简约风 / 跨境风共用）
    showTopMenuBar?: 0 | 1;
    /**
     * 顶部菜单文字颜色
     * 简约风：地区 / 会员菜单 / 站点导航
     * 跨境风：提示条右侧购物车 / 站点导航 + benefit 提示（主副标题以各自设置项优先）
     */
    topMenuTextColor?: string;
    /** 顶部菜单文字悬浮颜色，作用于两种风格的顶部菜单链接 */
    topMenuTextHoverColor?: string;
    showTopRegion?: 0 | 1;
    topRegionText?: string;
    showTopUserMenu?: 0 | 1;
    showTopMemberCenter?: 0 | 1;
    memberCenterItems?: PcHeaderMemberCenterItem[];
    showTopMemberName?: 0 | 1;
    showTopMessage?: 0 | 1;
    showTopOrders?: 0 | 1;
    showTopSiteNav?: 0 | 1;
    showTopAd?: 0 | 1;
    topAdPic?: PcHeaderPic;

    // LOGO
    logoPic?: PcHeaderPic;
    logoAlign?: 1 | 2 | 3;
    logoWidth?: number;
    logoHeight?: number;

    // 购物车
    rightCartShow?: 0 | 1;
    cartStyle?: 1 | 2;
    cartPic?: PcHeaderPic;
    topMenuCartShow?: 0 | 1;

    // 搜索条
    searchStyle?: 1 | 2 | 3;
    searchColorType?: 1 | 2;
    searchColor?: string;

    // 分类入口
    showCategory?: 0 | 1;
    /** 首页分类：0=隐藏，1=显示，仅首页在 showCategory=1 时生效 */
    showHomeCategory?: 0 | 1;
    categoryMarginTop?: number;

    /** 底部分隔样式：0=不展示，1=阴影，2=边框（与 Admin 一致） */
    bottomDividerType?: 0 | 1 | 2;

    // 顶部提示条（benefit bar）
    showBenefitBar?: 0 | 1;
    benefitBarBackgroundColor?: string;
    benefitTitleColor?: string;
    benefitDescColor?: string;
    benefitNoticeGroups?: PcHeaderNoticeGroup[];

    [key: string]: unknown;
}
