/** 礼品卡订单类型 */
export const GIFT_CARD_ORDER_TYPE = 11;
/** 实体礼品卡商品类型，需物流配送 */
export const PHYSICAL_GIFT_CARD_PRODUCT_TYPE = 7;

type GiftCardItem = { productType?: number | string | null } | null | undefined;
type GiftCardOrder = { orderType?: number | string | null; items?: GiftCardItem[] | null } | null | undefined;

/** 是否为实体礼品卡商品行 */
export const isPhysicalGiftCardItem = (item: GiftCardItem): boolean => Number(item?.productType ?? 0) === PHYSICAL_GIFT_CARD_PRODUCT_TYPE;

/** 礼品卡订单是否禁止售后：纯电子卡禁止，含实体卡放开 */
export const isGiftCardAftersalesBlocked = (order: GiftCardOrder): boolean => {
    const orderType = Number(order?.orderType ?? 0);
    if (orderType === 12) return true;
    if (orderType !== GIFT_CARD_ORDER_TYPE) return false;
    return !(order?.items ?? []).some(isPhysicalGiftCardItem);
};

/** 礼品卡订单中商品行是否可申请售后：仅实体卡可申请 */
export const canGiftCardItemApplyAftersales = (order: GiftCardOrder, item: GiftCardItem): boolean => {
    const orderType = Number(order?.orderType ?? 0);
    if (orderType === 12) return false;
    if (orderType !== GIFT_CARD_ORDER_TYPE) return true;
    return isPhysicalGiftCardItem(item);
};
