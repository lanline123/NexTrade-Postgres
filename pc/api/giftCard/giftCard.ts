import request from "@/utils/request";
import type { GiftCardExchangeParams, GiftCardGroupDetail, GiftCardInfo } from "@/types/giftCard/giftCard";

/**
 * @description 礼品卡兑换
 * @param params 包含卡号、卡密
 * @returns 兑换结果（可能返回商品订单或储值卡余额）
 */
export const giftCardExchange = (params: GiftCardExchangeParams) => {
    return asyncRequest({
        url: "promotion/giftCardGroups/exchange",
        method: "post",
        data: params
    });
};

/**
 * @description 获取礼品卡组详情
 * @param cardGroupsId 礼品卡组ID
 * @returns 礼品卡组信息及样式配置
 */
export const getGiftCardGroupDetail = (cardGroupsId: number) => {
    return request<GiftCardGroupDetail>({
        url: "promotion/giftCardGroups/detail",
        method: "get",
        params: { cardGroupsId }
    });
};

/**
 * @description 获取礼品卡详情
 * @param giftCardSn 礼品卡编号
 * @returns 礼品卡详情（状态、余额等）
 */
export const getGiftCardDetail = (giftCardSn: string) => {
    return asyncRequest<GiftCardInfo>({
        url: "promotion/giftCardGroups/detail",
        method: "get",
        params: { giftCardSn }
    });
};

/**
 * @description 获取礼品卡密码（需权限验证）
 * @param giftCardSn 礼品卡编号
 * @returns 礼品卡密码
 */
export const getGiftCardPassword = (giftCardSn: string) => {
    return asyncRequest({
        url: "promotion/giftCardGroups/showCardPassword",
        method: "get",
        params: { giftCardSn }
    });
};

/**
 * @description 礼品卡转赠（生成转赠链接）
 * @param data 包含礼品卡编号
 * @returns 转赠链接或二维码
 */
export const giftCardTransfer = (data: { giftCardSn: string; isFrom?: string }) => {
    return asyncRequest({
        url: "promotion/giftCardGroups/transfer",
        method: "post",
        data
    });
};

/**
 * @description 领取转赠的礼品卡
 * @param data 包含礼品卡编号
 * @returns 领取结果
 */
export const giftCardClaim = (data: { giftCardSn: string }) => {
    return request({
        url: "promotion/giftCardGroups/claim",
        method: "post",
        data
    });
};
