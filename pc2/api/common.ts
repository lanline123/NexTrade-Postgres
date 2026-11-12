import type { ApiPcCatFloorListWrapperVO } from "~/types/decoration/decorationClientVo";
import { request, asyncRequest } from "@/utils/request";
import type {
    LangListItem,
    DefaultLangItem,
    CurrencyItem,
    MobileAreaCodeList,
    ProductsListResponse,
    BaseConfigResponse,
    ThemeSettingsResponse
} from "@/types/common/common.d.ts";

/**
 * @description 获取PC端导航数据
 * @returns 导航配置
 */
export const getNav = () => {
    return request({
        url: "common/pc/getNav",
        method: "get"
    });
};

/**
 * @description 获取分类楼层数据
 * @returns 分类楼层配置
 */
export const getCatFloor = () => {
    return request<ApiPcCatFloorListWrapperVO>({
        url: "common/pc/getCatFloor",
        method: "get"
    });
};

/**
 * @description 获取搜索建议（模糊搜索）
 * @param keyword 搜索关键词
 * @returns 搜索建议列表
 */
export const getSearchGuessData = (keyword: string) => {
    return request<{ keyword: string }[]>({
        url: "search/searchGuess/index",
        method: "get",
        params: { keyword }
    });
};

export interface TopRegionResult {
    regionList: {
        regionName: string;
        regionId: number;
    }[];
}

/**
 * @description 获取省级地区列表
 * @returns 省份列表
 */
export const getTopRegion = () => {
    return asyncRequest<TopRegionResult>({
        url: "sys/region/getProvinceList",
        method: "get"
    });
};

/**
 * @description 获取用户当前地区
 * @returns 用户地区信息
 */
export const getUserRegion = () => {
    return asyncRequest<TopRegionResult>({
        url: "sys/region/getUserRegion",
        method: "get"
    });
};

/**
 * @description 获取系统基础配置
 * @param params 可选参数
 * @returns 完整系统配置
 */
export const getBaseConfig = (params?: any) => {
    return request<BaseConfigResponse>({
        url: "common/config/initConfigSettings",
        method: "get",
        params
    });
};

/**
 * @description 获取主题配置
 * @returns 主题样式配置
 */
export const getThemeSettings = () => {
    return request<ThemeSettingsResponse>({
        url: "common/config/themeSettings",
        method: "get"
    });
};

/**
 * @description 获取商品详情页风格
 * @returns 1-样式一（经典版/old），2-样式二（新版/new）
 */
export const getProductDetailStyle = () => {
    return request<number>({
        url: "common/config/productDetailStyle",
        method: "get"
    });
};

/**
 * @description 记录通用日志
 * @param params 日志参数
 * @returns 操作结果
 */
export const commonLog = (params: object) => {
    return asyncRequest({
        url: "common/log",
        method: "get",
        params
    });
};

/**
 * @description 获取语言包翻译数据
 * @param localeCode 语言代码
 * @returns 翻译数据
 */
export const getLang = (localeCode: string) => {
    return asyncRequest<Record<string, string>>({
        url: "common/i18n/getLocaleTranslations?localeCode=" + localeCode,
        method: "get"
    });
};

/**
 * @description 文本翻译
 * @param data 待翻译文本
 * @returns 翻译结果
 */
export const getTranslate = (data: object) => {
    return asyncRequest<any>({
        url: "common/translate/translate",
        method: "post",
        data
    });
};

/**
 * @description 获取支持的语言列表
 * @returns 语言列表
 */
export const getLangList = () => {
    return request<LangListItem[]>({
        url: "common/i18n/getLocales",
        method: "get"
    });
};

/**
 * @description 获取默认语言配置
 * @param code 语言代码
 * @returns 默认语言设置
 */
export const getDefaultLang = (code: string) => {
    return request<DefaultLangItem>({
        url: `common/i18n/getDefaultLocale?code=${code}`,
        method: "get"
    });
};

/**
 * @description 获取支持的货币列表
 * @returns 货币列表
 */
export const getCurrencyList = () => {
    return request<CurrencyItem[]>({
        url: "common/currency/getCurrency",
        method: "get"
    });
};

/**
 * @description 获取手机区号列表
 * @returns 国际区号列表
 */
export const getmobileAreaCode = () => {
    return request<MobileAreaCodeList[]>({
        url: "common/config/mobileAreaCode",
        method: "get"
    });
};

/**
 * @description 通用获取商品列表
 * @param params 筛选条件
 * @returns 商品列表
 */
export const getProductsList = (params: object) => {
    return asyncRequest<ProductsListResponse>({
        url: "product/product/list",
        method: "get",
        params
    });
};

export const getCategoryIdBySeoLink = (params: { seoLink: string }) => {
    return request<number>({
        url: "sys/seo/getCategoryIdBySeoLink",
        method: "get",
        params
    });
};
