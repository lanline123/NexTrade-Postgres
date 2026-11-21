import request from "@/utils/request";

/**
 * 充值赠品（我方新增，2026-11-21 论坛帖 717721）
 * 充值档位配置实物赠品后，会员充值到账生成待领取记录，在此领取。
 */

export interface RechargeGiftRecord {
    recordId: number;
    rechargeOrderId: number;
    giftId: number;
    giftName: string;
    giftPic?: string;
    quantity: number;
    /** 0 待领取 / 1 已领取 / 2 已作废 / 3 异常 */
    status: number;
    statusText: string;
    orderId: number;
    consignee?: string;
    mobile?: string;
    addressText?: string;
    adminNote?: string;
    addTime: number;
    claimTime: number;
}

/**
 * @description 我的赠品记录列表
 */
export const getMyRechargeGifts = (params?: object) => {
    return request<any>({
        url: "user/rechargeGift/list",
        method: "get",
        params
    });
};

/**
 * @description 领取赠品（选收货地址，生成赠品履约订单）
 */
export const claimRechargeGift = (data: { recordId: number; addressId: number }) => {
    return request<any>({
        url: "user/rechargeGift/claim",
        method: "post",
        data
    });
};
