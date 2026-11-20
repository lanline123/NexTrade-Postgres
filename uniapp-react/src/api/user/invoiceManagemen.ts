import request from "@/utils/request";

/**
 * @description 申请订单电子发票
 * @param data 发票信息
 * @returns 操作结果
 */
export const orderInvoiceInsert = (data: object) => {
    return request({
        url: "user/orderInvoice/create",
        method: "post",
        data
    });
};

/**
 * @description 更新订单发票信息
 * @param data 发票信息
 * @returns 操作结果
 */
export const orderInvoiceUpdate = (data: object) => {
    return request({
        url: "user/orderInvoice/update",
        method: "post",
        data
    });
};

/**
 * @description 获取订单发票申请详情
 * @param orderId 订单ID（接口入参名为 id）
 * @returns 订单发票申请详情
 */
export const getOrderInvoiceDetail = (orderId: number | string) => {
    return request<any>({
        url: "user/orderInvoice/detail",
        method: "get",
        params: { id: orderId }
    });
};

/**
 * @description 获取用户增值税发票资质信息
 * @returns 增票资质详情
 */
export const getUserInvoice = () => {
    return request<any>({
        url: "user/invoice/detail",
        method: "get"
    });
};

/**
 * @description 创建增值税发票资质
 * @param data 资质信息
 * @returns 操作结果
 */
export const createUserInvoice = (data: object) => {
    return request({
        url: "user/invoice/create",
        method: "post",
        data
    });
};

/**
 * @description 更新增值税发票资质
 * @param data 资质信息
 * @returns 操作结果
 */
export const updateUserInvoice = (data: object) => {
    return request<any>({
        url: "user/invoice/update",
        method: "post",
        data
    });
};

/**
 * @description 获取增票资质审核状态
 * @returns 审核状态
 */
export const getInvoiceStatus = () => {
    return request<any>({
        url: "user/invoice/getStatus",
        method: "get"
    });
};

/**
 * @description 常用发票抬头列表（v5.8.65 跟进，默认在前）
 * @returns 抬头列表
 */
export const getInvoiceTitleList = () => {
    return request<any>({
        url: "user/invoiceTitle/list",
        method: "get"
    });
};

/**
 * @description 新建常用发票抬头
 * @param data 抬头信息
 * @returns 新抬头ID
 */
export const createInvoiceTitle = (data: object) => {
    return request<any>({
        url: "user/invoiceTitle/create",
        method: "post",
        data
    });
};

/**
 * @description 更新常用发票抬头
 * @param data 抬头信息（含 id）
 * @returns 操作结果
 */
export const updateInvoiceTitle = (data: object) => {
    return request<any>({
        url: "user/invoiceTitle/update",
        method: "post",
        data
    });
};

/**
 * @description 删除常用发票抬头（软删，删默认自动递补）
 * @param id 抬头ID
 * @returns 操作结果
 */
export const deleteInvoiceTitle = (id: number) => {
    return request<any>({
        url: "user/invoiceTitle/del",
        method: "post",
        data: { id }
    });
};

/**
 * @description 设置默认发票抬头
 * @param id 抬头ID
 * @returns 操作结果
 */
export const setDefaultInvoiceTitle = (id: number) => {
    return request<any>({
        url: "user/invoiceTitle/setDefault",
        method: "post",
        data: { id }
    });
};
