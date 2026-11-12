// 首页
import type { CouponFilterState } from "~/types/user/coupon";

export interface HomeDataResult {
    decorateId: number;
    moduleList: ModuleListType[];
    couponList: CouponFilterState[];
    /** 页面级配置，含 homeStyleVersion（新旧首页分流）与页面背景设置 */
    pageModule?: Record<string, any> | null;
}
export interface ModuleListType {
    module: {};
    moduleIndex: number;
    isShow: boolean;
    active: boolean;
    type: string;
    style: number;
}

export interface CouponResponse {
    isReceive: number;
    receiveNum: number;
    couponId: number;
    couponName: string;
    couponMoney: number;
    couponDiscount: number;
    couponDesc: string;
    couponType: number;
    sendRange: number;
    sendRangeData: number[];
    minOrderAmount: number;
    sendStartDate: string;
    sendEndDate: string;
    sendType: number;
    useDay: number;
    useStartDate: string;
    useEndDate: string;
    isShow: number;
    isGlobal: number;
    isNewUser: number;
    enabledClickGet: number;
    limitUserRank: number[];
    shopId: number;
    isDelete: number;
    limitNum: number;
    delayDay: number;
    sendNum: number;
    maxOrderAmount: number;
    couponUnit: number;
    reduceType: number;
    addTime: string;
}

// 装修分类模块使用 getRecommend 的 catList 返回值。
export interface HomeCategoryRecommendParams {
    decorateId: number;
    previewId?: number;
    moduleIndex: number;
}

export interface HomeCategoryRecommendData {
    catList: {
        categoryId: number;
        categoryName: string;
        categoryPic: string;
        seoLink?: string;
    }[];
}
