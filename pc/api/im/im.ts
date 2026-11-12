import request, { asyncRequest } from "~/utils/request";

/** IM接口请求前缀 */
const IM_PREFIX = "/im/";

/**
 * 获取会话列表
 * @description 获取用户的IM会话列表
 * @param params - 分页参数
 * @returns 会话列表
 */
export const getConversation = (params: { page?: number; size?: number }) => {
    return asyncRequest<any>({
        prefix: IM_PREFIX,
        url: "conversation/conversation/list",
        method: "get",
        params
    });
};

/**
 * 获取会话消息列表
 * @description 获取指定会话的消息历史
 * @param params - 会话参数（店铺ID、会话ID等）
 * @returns 消息列表
 */
export const getConversationMessage = (params: {
    shopId: number;
    conversationId?: number;
    firstId?: number;
    sortOrder?: string;
}) => {
    return asyncRequest<any>({
        prefix: IM_PREFIX,
        url: "conversation/message/list",
        method: "get",
        params
    });
};

/**
 * 发送IM消息
 * @description 向指定会话发送消息（文本、图片、商品卡片等）
 * @param data - 消息内容和目标信息
 * @returns 发送结果
 */
export const sendMessage = (data: {
    content: {
        messageType: string;
        content?: string;
        pic?: string;
        product?: any;
        order?: any;
    };
    conversationId?: number;
    shopId: number;
    productId?: number;
    orderId?: number;
}) => {
    return asyncRequest<any>({
        prefix: IM_PREFIX,
        url: "conversation/message/send",
        method: "post",
        data
    });
};

/**
 * 设置消息已读
 * @description 标记会话消息为已读状态
 * @param data - 店铺ID和客服ID
 * @returns 操作结果
 */
export const setRead = (data: { shopId: number; servantId: number }) => {
    return asyncRequest<any>({
        prefix: IM_PREFIX,
        url: "conversation/message/setRead",
        method: "post",
        data
    });
};

/**
 * 获取店铺详情（IM用）
 * @description 在IM场景下获取店铺信息
 * @param id - 店铺ID
 * @returns 店铺详情
 */
export const getShopDetail = (id: number) => {
    return request<any>({
        url: `shop/shop/detail?shopId=${id}`,
        method: "get"
    });
};


/**
 * 搜索会话
 * @description 根据关键词筛选会话列表
 * @param params - 搜索参数
 * @returns 匹配的会话列表
 */
export const frontSearch = (params:object) => {
    return asyncRequest<any>({
        prefix: IM_PREFIX,
        url: "conversation/conversation/frontSearch",
        method: "get",
        params
    });
};