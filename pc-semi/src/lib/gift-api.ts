import { apiGet, apiPost } from '@/lib/client-api';

/**
 * 送礼物订单流 API（我方新增，论坛帖 197742）。
 *
 * 领取页与「我收到的礼物」的数据结构刻意不含任何价格字段
 * （送礼语义，领取人不得见付款金额），类型与后端 VO 对齐。
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

export const giftClaimPreview = (token: string) =>
  apiPost<GiftClaimPreviewVO>('order/gift/claimPreview', { token });

export const giftClaim = (token: string, addressId: number) =>
  apiPost<GiftInfoVO>('order/gift/claim', { token, addressId });

export const getGiftInfo = (orderId: number) =>
  apiGet<GiftInfoVO | null>('user/order/giftInfo', { orderId });

export const regenerateGiftShareToken = (orderId: number) =>
  apiPost<{ token: string; claimPath: string }>('user/order/giftShareToken', { orderId });

export const cancelGiftOrder = (orderId: number) =>
  apiPost<boolean>('user/order/giftCancel', { orderId });

export const getReceivedGifts = () => apiGet<GiftReceivedVO[]>('user/gift/received');
