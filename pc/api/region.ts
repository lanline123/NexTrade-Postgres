import { request, asyncRequest } from "@/utils/request";
import type { RegionList } from "~/types/user/address.d";
import type { ByLocationData } from "@/types/region";

export interface CountryRegion extends RegionList {
    standardCode: string;
    mobileAreaCode: string;
    regionInputMode?: "STATIC" | "DYNAMIC";
    nextLevelAvailable?: boolean;
}

/**
 * 根据ID获取地区信息
 * @description 批量获取地区信息（省市区名称）
 * @param ids - 地区ID字符串（逗号分隔）
 * @returns 地区列表
 */
export const getRegionByIds = (ids: string) => {
    return asyncRequest<RegionList[]>({
        url: "sys/region/getRegion",
        method: "get",
        params: { regionIds: ids }
    });
};

/** 获取可用国家/地区根节点（跨境地址）。 */
export const getCountryList = () =>
    asyncRequest<CountryRegion[]>({
        url: "sys/region/getCountryList",
        method: "get"
    });

/** 获取指定地区的下一级节点（跨境地址动态联动）。 */
export const getRegionListByRegionId = (regionId: number | string) =>
    asyncRequest<CountryRegion[]>({
        url: "sys/region/getRegionListByRegionId",
        method: "get",
        params: { regionId: Number(regionId) }
    });

/**
 * 通过GPS获取定位
 * @description 根据经纬度获取所在城市
 * @param location - 经纬度坐标字符串
 * @returns 定位结果
 */
export const getLocationByLocation = (location: string) => {
    return asyncRequest<ByLocationData>({
        url: `sys/region/getLocationByLocation?location=${location}`,
        method: "get"
    });
};

/**
 * 通过IP获取定位
 * @description 根据客户端IP获取所在城市
 * @returns 定位结果
 */
export const getLocationByIp = () => {
    return request<ByLocationData>({
        url: "sys/region/getLocationByIp",
        method: "get"
    });
};

/**
 * 获取地区路径
 * @description 根据地区ID获取完整的省市区路径
 * @param regionId - 地区ID
 * @returns 地区路径数组
 */
export const getRegionPath = (regionId: number) => {
    return request<any[]>({
        url: `sys/region/getRegionPath?regionId=${regionId}`,
        method: "get"
    });
};
