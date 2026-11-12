import { asyncRequest } from "~/utils/request";
import type {
    AccountDetailFilterResult,
    AccountFilterParams,
    AccountFilterResult,
    AccountFormResult,
    AccountInfo,
    AccountInfoResult,
    DepositFilterResult,
    DepositFilterState
} from "~/types/user/account.d";

/**
 * 获取充值订单列表
 * @description 获取用户的余额充值订单记录
 * @param params - 分页参数
 * @returns 充值订单列表
 */
export const getRechargeOrderList = (params?: AccountFilterParams) => {
    return asyncRequest<AccountFilterResult>({
        url: "user/rechargeOrder/list",
        method: "get",
        params
    });
};

/**
 * 获取账户变动记录
 * @description 获取用户余额的收支明细
 * @param params - 分页参数
 * @returns 账户变动列表
 */
export const getAccountList = (params?: AccountFilterParams) => {
    return asyncRequest<AccountDetailFilterResult>({
        url: "user/account/list",
        method: "get",
        params
    });
};

/**
 * 获取充值档位设置
 * @description 获取可选的充值金额档位
 * @returns 充值档位列表
 */
export const getDepositList = () => {
    return asyncRequest<DepositFilterState[]>({
        url: "user/rechargeOrder/setting",
        method: "get"
    });
};

/**
 * 更新充值订单
 * @description 更新充值订单信息
 * @param data - 订单数据
 * @returns 操作结果
 */
export const updateRechargeOrder = (data: object) => {
    return asyncRequest({
        url: "user/rechargeOrder/update",
        method: "post",
        data
    });
};

/**
 * 充值订单支付
 * @description 对充值订单发起支付
 * @param data - 支付参数
 * @returns 支付结果
 */
export const updateRechargeOrderPay = (data: object) => {
    return asyncRequest<any>({
        url: "user/rechargeOrder/pay",
        method: "post",
        data
    });
};

/**
 * 创建充值订单
 * @description 创建新的充值订单
 * @param data - 充值参数（金额等）
 * @returns 创建的订单信息
 */
export const updateRechargeOrderCreate = (data: object) => {
    return asyncRequest<any>({
        url: "user/rechargeOrder/create",
        method: "post",
        data
    });
};

/**
 * 获取可用支付方式
 * @description 获取充值可用的支付方式列表
 * @returns 支付方式列表
 */
export const getPaymentList = () => {
    return asyncRequest<string[]>({
        url: "user/rechargeOrder/paymentList",
        method: "get"
    });
};

/**
 * 提现账户操作
 * @description 执行提现账户相关操作
 * @param data - 操作数据
 * @param url - 操作类型
 * @returns 操作结果
 */
export const updateAccount = (data: object, url: string) => {
    return asyncRequest({
        url: "user/withdrawApply/" + url,
        method: "post",
        data
    });
};

/**
 * 获取提现申请列表
 * @description 获取用户的提现申请记录
 * @param params - 分页参数
 * @returns 提现申请列表
 */
export const getAccountNoList = (params?: AccountFilterParams) => {
    return asyncRequest<AccountInfoResult>({
        url: "user/withdrawApply/list",
        method: "get",
        params
    });
};

/**
 * 获取提现账户详情
 * @description 获取用户的提现账户信息
 * @param params - 查询参数
 * @returns 账户详情
 */
export const getAccount = (params?: object) => {
    return asyncRequest<AccountFormResult>({
        url: "user/withdrawApply/accountDetail",
        method: "get",
        params
    });
};

/**
 * 删除提现账户
 * @description 删除用户的提现账户
 * @param params - 账户ID
 * @returns 操作结果
 */
export const delAccount = (data: object) => {
    return asyncRequest({
        url: "user/withdrawApply/delAccount",
        method: "post",
        data
    });
};
/**
 * 提交提现申请
 * @description 申请将账户余额提现到银行卡/支付宝
 * @param data - 提现参数（金额、账户等）
 * @returns 申请结果
 */
export const updateWithdrawApply = (data: object) => {
    return asyncRequest({
        url: "user/withdrawApply/apply",
        method: "post",
        data
    });
};

/**
 * 检查充值支付状态
 * @description 轮询检查充值订单是否支付成功
 * @param params - 订单参数
 * @returns 支付状态
 */
export const checkUserPayStatus = (params: {}) => {
    return asyncRequest({
        url: "user/rechargeOrder/checkStatus",
        method: "get",
        params
    });
};
