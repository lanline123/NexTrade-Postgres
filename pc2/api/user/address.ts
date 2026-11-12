import { asyncRequest } from "~/utils/request";
import type { AddressFilterParams, AddressFilterResult, AddressFormState } from "~/types/user/address.d";

/**
 * 获取收货地址列表
 * @description 获取用户的所有收货地址
 * @param params - 分页参数
 * @returns 地址列表
 */
export const getAddressList = (params: AddressFilterParams) => {
    return asyncRequest<AddressFilterResult>({
        url: "user/address/list",
        method: "get",
        params: params
    });
};

/**
 * 删除收货地址
 * @description 删除指定的收货地址
 * @param data - 地址ID
 * @returns 操作结果
 */
export const delAddress = (data: object) => {
    return asyncRequest({
        url: "user/address/del",
        method: "post",
        data
    });
};

/**
 * 获取收货地址详情
 * @description 获取单个地址的详细信息（用于编辑）
 * @param action - 操作类型（detail等）
 * @param params - 地址ID
 * @returns 地址详情
 */
export const getAddressData = (action: string, params: object) => {
    return asyncRequest<AddressFormState>({
        url: "user/address/" + action,
        method: "get",
        params
    });
};

/**
 * 更新收货地址
 * @description 新增或修改收货地址
 * @param operation - 操作类型（add/update）
 * @param data - 地址数据
 * @returns 操作结果
 */
export const updateAddressData = (operation: string, data: object) => {
    return asyncRequest<any>({
        url: "user/address/" + operation,
        method: "post",
        data
    });
};

/**
 * 设置默认收货地址
 * @description 将指定地址设为默认地址
 * @param id - 地址ID
 * @returns 操作结果
 */
export const setSelectedAddress = (id: number) => {
    return request({
        url: `user/address/setSelected`,
        method: "post",
        data: { id }
    });
};
