import request from "@/utils/request";
import type { ApiEnvelope } from "@/types/api";

/**
 * 后厨出单与催菜 API（我方新增，论坛帖 817927）。
 *
 * 买家在订单详情查看后厨进度并催菜；频控（10 分钟冷却 +
 * 每日 5 次）在后端，`kitchenInfo` 下发当前可否催菜与冷却
 * 剩余秒数，前端据此置灰按钮。
 */

export interface KitchenInfoVO {
    orderId: number;
    /** 订单是否在后厨口径内（已付款待发货） */
    inKitchen: boolean;
    /** 0 待制作 / 1 制作中 / 2 已出单 */
    kitchenStatus: number;
    kitchenStatusName: string;
    urgeCount: number;
    canUrge: boolean;
    cooldownRemainSeconds: number;
    dailyUrgesLeft: number;
}

export interface KitchenUrgeResultVO {
    orderId: number;
    urgeCount: number;
    cooldownSeconds: number;
}

/** 订单详情后厨面板（状态 + 催菜频控现状） */
export const getKitchenInfo = (orderId: number) => {
    return request<ApiEnvelope<KitchenInfoVO>>({
        url: "user/order/kitchenInfo",
        method: "get",
        params: { orderId }
    });
};

/** 催菜（频控不通过时后端返回业务错误文案） */
export const urgeOrder = (orderId: number) => {
    return request<ApiEnvelope<KitchenUrgeResultVO>>({
        url: "user/order/urge",
        method: "post",
        data: { orderId }
    });
};
