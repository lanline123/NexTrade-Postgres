import request from "@/utils/request";
import type { ApiEnvelope } from "@/types/api";

/**
 * 送礼物订单流 API（我方新增，论坛帖 197742）。
 *
 * 注意：领取页与「我收到的礼物」的数据结构刻意不含任何价格字段
 * （送礼语义，领取人不得见付款金额），类型定义与后端 VO 对齐。
 */

export interface GiftItemVO {
    itemId: number;
    orderId: number;
    productId: number;
    productName: string;
    picThumb: string;
    skuData: string;
    quantity: number;
}

export interface GiftReceivedUnitVO {
    orderId: number;
    orderSn: string;
    shopId: number;
    orderStatus: number;
    orderStatusName: string;
    trackingNo: string;
    logisticsName: string;
    items: GiftItemVO[];
}

export interface GiftClaimPreviewVO {
    giftStatus: number;
    giftStatusName: string;
    giftMessage: string;
    buyerNameMasked: string;
    expireTime?: string | null;
    expired: boolean;
    canClaim: boolean;
    items: GiftItemVO[];
    units: GiftReceivedUnitVO[];
}

export interface GiftInfoVO {
    orderId: number;
    giftStatus: number;
    giftStatusName: string;
    giftMessage: string;
    expireTime?: string | null;
    claimUserNameMasked?: string | null;
    claimTime?: string | null;
    closeReason: string;
}

export interface GiftReceivedVO {
    orderId: number;
    orderSn: string;
    giftStatus: number;
    giftStatusName: string;
    giftMessage: string;
    buyerNameMasked: string;
    claimTime?: string | null;
    units: GiftReceivedUnitVO[];
}

/** 领取页预览（可匿名；无价格） */
export const giftClaimPreview = (token: string) => {
    return request<ApiEnvelope<GiftClaimPreviewVO>>({
        url: "order/gift/claimPreview",
        method: "post",
        data: { token }
    });
};

/** 领取（必须登录，addressId 为领取人自己的地址） */
export const giftClaim = (token: string, addressId: number) => {
    return request<ApiEnvelope<GiftInfoVO>>({
        url: "order/gift/claim",
        method: "post",
        data: { token, addressId }
    });
};

/** 买家订单礼物面板 */
export const getGiftInfo = (orderId: number) => {
    return request<ApiEnvelope<GiftInfoVO | null>>({
        url: "user/order/giftInfo",
        method: "get",
        params: { orderId }
    });
};

/** 重新生成分享链接（旧链接作废），返回新 token 与领取路径 */
export const regenerateGiftShareToken = (orderId: number) => {
    return request<ApiEnvelope<{ token: string; claimPath: string }>>({
        url: "user/order/giftShareToken",
        method: "post",
        data: { orderId }
    });
};

/** 买家取消礼物订单（未付款直取消 / 待领取退款关闭） */
export const cancelGiftOrder = (orderId: number) => {
    return request<ApiEnvelope<boolean>>({
        url: "user/order/giftCancel",
        method: "post",
        data: { orderId }
    });
};

/** 我收到的礼物（领取人视角，无价格） */
export const getReceivedGifts = () => {
    return request<ApiEnvelope<GiftReceivedVO[]>>({
        url: "user/gift/received",
        method: "get"
    });
};
