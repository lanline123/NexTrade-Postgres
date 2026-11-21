import request from "@/utils/request";
import type { GiftCardDetailResponse, GiftCardExchangeParams } from "@/types/giftCard/giftCard";

/**
 * @description 礼品卡兑换
 * @param params 包含卡号、卡密
 * @returns 兑换结果（储值卡充值到余额/兑换商品进入购物流程）
 */
export const giftCardExchange = (params: GiftCardExchangeParams) => {
    return request({
        url: "promotion/giftCardGroups/exchange",
        method: "post",
        data: params
    });
};

/**
 * @description 获取礼品卡详情
 * @param cardNum 礼品卡卡号
 * @param giftCardSn 卡唯一序列号
 * @returns 礼品卡信息（面额、类型、有效期、状态）
 */
export const getGiftCardDetail = (cardNum: string, giftCardSn: string) => {
    return request<GiftCardDetailResponse>({
        url: "promotion/giftCardGroups/detail",
        method: "get",
        params: { cardNum, giftCardSn }
    });
};

/**
 * @description 查看礼品卡密码
 * @param cardNum 礼品卡卡号
 * @param giftCardSn 卡唯一序列号
 * @returns 礼品卡密码（脱敏处理）
 */
export const getGiftCardPassword = (cardNum: string, giftCardSn: string) => {
    return request({
        url: "promotion/giftCardGroups/showCardPassword",
        method: "get",
        params: { cardNum, giftCardSn }
    });
};

/** 我的礼品卡（P1） */
export interface MyGiftCardItem {
    giftCardsId: number;
    cardGroupsId: number;
    cardGroupName: string;
    imageUrl?: string;
    groupType: number;
    cardType: number;
    cardNumber: string;
    giftCardSn?: string;
    faceAmount?: string;
    balance?: string;
    status: number;
    stateText: string;
    isValid: boolean;
    sourceText: string;
    expiredTime: number;
    usedTime?: number;
    addTime?: number;
    /** 适用范围（我方新增，后端下发）：0 全部商品 / 1 指定商品 / 2 指定品类 */
    scopeType?: number;
    /** 适用范围文案：全场通用 / 仅限指定商品 / 仅限指定品类 */
    scopeDesc?: string;
}

export type MyGiftCardDetail = MyGiftCardItem & {
    usageLogs: Array<{
        id: number;
        typeName: string;
        amount: string;
        balanceAfter?: string;
        orderId: number;
        addTime: number;
    }>;
    exchangeOrderIds: number[];
    /** 兑换模式（兑换卡，后端下发后才有，宽松读取）：1 固定组合 / 2 任选 */
    exchangeMode?: number;
    /** 任选件数（兼容 maxSelectCount 兜底） */
    selectCount?: number;
    maxSelectCount?: number;
    /** 兑换卡绑定商品池（任选选品用，字段宽松读取） */
    giftCardProducts?: Array<{
        productId: number;
        skuId: number;
        productName?: string;
        picUrl?: string;
        skuData?: string;
        skuPrice?: number | string;
        productPrice?: number | string;
        skuStock?: number;
        productStock?: number;
        stock?: number;
    }>;
};

/**
 * @description 使用/核销兑换卡（固定组合无 items；任选带 items 按 SKU 计件）
 */
export const useGiftCard = (data: {
    giftCardsId: number;
    items?: Array<{ productId: number; skuId: number; quantity: number }>;
}) => {
    return request({
        url: "promotion/giftCardGroups/useCard",
        method: "post",
        data
    });
};

/**
 * @description 我的礼品卡列表（P1；type 1 储值/2 兑换，state valid/invalid）
 */
export const getMyGiftCards = (params: { page?: number; size?: number; type?: number; state?: string }) => {
    return request<{ code: number; message: string; data: { records: MyGiftCardItem[]; total: number } }>({
        url: "promotion/giftCardGroups/myCards",
        method: "get",
        params
    });
};

/**
 * @description 我的礼品卡详情（P1；归属校验在后端）
 */
export const getMyGiftCardDetail = (giftCardsId: number) => {
    return request<{ code: number; message: string; data: MyGiftCardDetail }>({
        url: "promotion/giftCardGroups/myCardDetail",
        method: "get",
        params: { giftCardsId }
    });
};

/**
 * @description 礼品卡转赠给他人
 * @param data 包含礼品卡序列号及转赠来源标识
 * @returns 转赠结果（生成分享链接/二维码）
 */
export const giftCardTransfer = (data: { giftCardSn: string; isFrom: string }) => {
    return request({
        url: "promotion/giftCardGroups/transfer",
        method: "post",
        data
    });
};

/**
 * @description 领取他人转赠的礼品卡
 * @param data 包含卡号及序列号
 * @returns 领取结果，礼品卡绑定到当前用户
 */
export const giftCardClaim = (data: { cardNum: string; giftCardSn: string }) => {
    return request({
        url: "promotion/giftCardGroups/claim",
        method: "post",
        data
    });
};
