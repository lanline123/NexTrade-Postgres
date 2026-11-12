import { asyncRequest } from "@/utils/request";
import type { UserCreditDetailVO } from "~/types/user/termPay";

/**
 * 获取当前登录用户的账期支付信息详情（额度信息 + 未结算/已结算订单列表）
 * GET /api/user/user/creditDetail
 */
export const getUserCreditDetail = () => {
    return asyncRequest<UserCreditDetailVO>({
        url: "user/user/creditDetail",
        method: "get"
    });
};

