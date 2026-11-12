import type { MoneyAmount, Timestamp } from "~/types/api";

/** 对齐 ApiPcCatFloorListCategoryVO.ChildCategoryVO（递归） */
export interface ApiPcCatFloorListCategoryChildCategoryVO {
    id?: number;
    /** @i18n */
    name?: string;
    /** @i18n */
    categoryPic?: string;
    children?: ApiPcCatFloorListCategoryChildCategoryVO[];
}

/** 对齐 `decorate/application/vo/api/ApiDecorateVO` */
export interface ApiDecorateVO {
    id?: number;
    data?: unknown;
    decorateType?: number;
    pageType?: number;
}

/** 对齐 `decorate/application/vo/api/ApiPcHeaderVO` */
export interface ApiPcHeaderVO {
    pcHeader?: unknown;
}

/** PC 登录页装修配置内容（对齐 Admin pcLogin 的 module 字段） */
export interface ApiPcLoginModuleConfig {
    /** 登录页模式：1=经典(BaseLogin) / 2=现代(OverseasLogin) */
    loginMode?: 1 | 2 | string | number;
    /** 背景图 */
    backgroundPic?: {
        picUrl?: string;
        picThumb?: string;
    };
    /** 背景色 */
    backgroundColor?: string;
    /** 头部高度，单位 px */
    headerHeight?: number | string;
    /** Logo 宽度，单位 px（配合 object-fit: contain 防止变形） */
    logoWidth?: number | string;
    /** Logo 高度，单位 px（配合 object-fit: contain 防止变形） */
    logoHeight?: number | string;
    /** 头部是否显示阴影 */
    headerShadow?: boolean | number | string;
    /** 头部是否显示欢迎语与登录/注册互链 */
    showWelcome?: boolean | number | string;
    /** 登录区(login-warp)最小高度，单位 px，取值 620–800 */
    warpHeight?: number | string;
    [key: string]: unknown;
}

/** moduleList 单项：业务字段在 module 内，与搜索页 pcSearchResult 结构一致 */
export interface ApiPcLoginModuleItemVO {
    type?: string;
    module?: ApiPcLoginModuleConfig;
    [key: string]: unknown;
}

/** 对齐 `home/home/pcLogin` 返回（DecorateModuleListVO，配置在 moduleList[].module） */
export interface ApiPcLoginDecorateVO {
    moduleList?: ApiPcLoginModuleItemVO[];
    pageModule?: unknown;
    isHome?: number;
}

/** @deprecated 请使用 ApiPcLoginModuleConfig；保留别名避免旧引用报错 */
export type ApiPcLoginModuleVO = ApiPcLoginModuleConfig;

/** 对齐 `home/application/vo/ApiHomeConfigCustomerVO` */
export interface ApiHomeConfigCustomerVO {
    h5Domain?: string;
    corpId?: number;
    url?: string;
    openType?: number;
    serviceType?: number;
    show?: number;
}

/** 对齐 `pccatfloor/.../CategoryTitleDTO` */
export interface ApiPcCatFloorCategoryTitle {
    categoryTitleName?: string;
    categoryLink?: string;
}

/** 对齐 `pccatfloor/.../CategoryTitleVO` */
export type CategoryTitleVO = ApiPcCatFloorCategoryTitle;

/** 对齐 `pccatfloor/.../ApiPcCatFloorProductVO` */
export interface ApiPcCatFloorProductVO {
    productId?: number;
    /** @i18n */
    productName?: string;
    /** @i18n */
    picThumb?: string;
    productPrice?: MoneyAmount;
    marketPrice?: MoneyAmount;
}

/** 对齐 `pccatfloor/.../ApiPcCatFloorListCategoryVO` */
export interface ApiPcCatFloorListCategoryVO {
    id?: number;
    /** @i18n */
    name?: string;
    /** @i18n */
    pic?: string;
    isLeaf?: boolean;
    children?: ApiPcCatFloorListCategoryChildCategoryVO[];
    products?: ApiPcCatFloorProductVO[];
}

/** 对齐 `pccatfloor/.../ApiPcCatFloorListVO` */
export interface ApiPcCatFloorListVO {
    catFloorId?: number;
    catFloorNames?: ApiPcCatFloorCategoryTitle[];
    floorIcoFont?: string;
    useImage?: number;
    imageUrl?: string;
    catList?: ApiPcCatFloorListCategoryVO[];
}

/** 对齐 `pccatfloor/.../ApiPcCatFloorListWrapperVO` */
export interface ApiPcCatFloorListWrapperVO {
    catFloor?: ApiPcCatFloorListVO[];
    icoDefinedCss?: string;
    categoryDrawerStyle?: number;
    categoryDrawerMode?: number;
}

/** 对齐 `pcnavigation/.../ApiNavItemVO` */
export interface ApiNavItemVO {
    id?: number;
    parentId?: number;
    title?: string;
    sortOrder?: number;
    type?: number;
    isShow?: number;
    isBlank?: number;
    link?: unknown;
    icon?: string;
    children?: ApiNavItemVO[];
}

/** 对齐 `pcnavigation/.../ApiNavVO` */
export interface ApiNavVO {
    mainNav?: ApiNavItemVO[];
    topBarNav?: ApiNavItemVO[];
    bottomNav?: ApiNavItemVO[];
    sidebarNav?: ApiNavItemVO[];
}

/** 对齐 `decoratediscrete/application/vo/DecorateDiscreteVO`（C 端引用） */
export interface DecorateDiscreteVO {
    id?: number;
    decorateSn?: string;
    decorateName?: string;
    shopId?: number;
    data?: unknown;
}

/** 对齐 `decorateshare/application/vo/DecorateShareVO` */
export interface DecorateShareVO {
    shareSn?: string;
    shareToken?: string;
    decorateId?: number;
    validTime?: Timestamp;
    shareUrl?: string;
    sn?: string;
    token?: string;
    apiUrl?: string;
    decorateShareName?: string;
    decorateSharePic?: string;
    sortOrder?: number;
    isShow?: number;
    status?: number;
    decorateShareDesc?: string;
}
