import { request, asyncRequest } from "@/utils/request";
import type { bindMobileResult } from "~/types/user/login";
// 获取购物车
export const userRegist = (data: {}) => {
    return asyncRequest({
        url: "user/regist/registAct",
        method: "post",
        data
    });
};

export const sendMobileCode = (data: {}) => {
    return asyncRequest({
        url: "user/login/sendMobileCode",
        method: "post",
        data
    });
};

export const sendEmailCode = (data: {}) => {
    return asyncRequest({
        url: "user/login/sendEmailCode",
        method: "post",
        data
    });
};

export const userBindMobile = (data: {}) => {
    return asyncRequest<bindMobileResult>({
        url: "user/login/bindMobile",
        method: "post",
        data
    });
};
