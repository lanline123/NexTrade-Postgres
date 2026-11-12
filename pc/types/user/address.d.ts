import type { BaseResponseListWrap } from "~/types/api";
export interface AddressFilterParams {
    page: number;
    size: number;
}

// 获取列表返回参数类型
export interface AddressFilterState {
    consignee: string; // 收货人姓名
    regionName: string; // 地区名
    addressId: number; //
    address: string; // 详细地址
    email: string; // 电子邮箱
    telephone: string; // 电话号码
    mobile: string; // 手机号码
    positionAddress: string; // 定位地址
    countryCode?: string;
    countryName?: string;
    mobileAreaCode?: string;
    internationalMobile?: string;
    regionIds?: number[];
    regionNames?: string[];
    addressData?: Record<string, string>;
    formattedAddress?: string;
    fullAddress?: string;
    postcode?: string;
    isInvalid?: boolean;
}
//详情参数
export interface AddressFormState {
    consignee: string; // 收货人姓名
    regionIds: number[]; // 地区id
    regionName?: string; // 地区名
    addressId?: number; //
    address: string; // 详细地址
    postcode?: string; // 邮政编码
    positionAddress?: string; // 定位地址
    longitude: number; // 经度
    latitude: number; // 纬度
    email: string; // 电子邮箱
    telephone: string; // 电话号码
    mobile: string; // 手机号码
    userId?: number;
    isSelected?: number;
    isDefault?: number;
    countryCode?: string;
    countryName?: string;
    mobileAreaCode?: string;
    internationalMobile?: string;
    regionNames?: string[];
    addressData?: Record<string, string>;
    addressLineTwo?: string;
    buildingName?: string;
    floor?: string;
    unit?: string;
    formattedAddress?: string;
    fullAddress?: string;
    isInvalid?: boolean;
}
export interface AddressFilterResult extends BaseResponseListWrap<AddressFilterState[]> {
    filter: AddressFilterParams;
    optimizeCountSql: AddressFilterResult;
    searchCount: AddressFilterResult;
}

export interface AddressFormResult {
    item: AddressFormState;
}

export interface RegionFilterState {
    errcode: number;
    records: Array<RegionList[]>;
    message: string;
}

export interface RegionList {
    regionId: number;
    regionName: string;
    parentId: number;
    level: number;
    isHot: number;
    firstWord: string;
}
