import { asyncRequest } from "~/utils/request";
// 老手机获得验证码
export const sendMobileCodeByMobileValidate = (data: object) => {
    return asyncRequest({
        url: "user/user/sendMobileCodeByMobileValidate",
        method: "post",
        data
    });
};
// 修该手机老手机验证码验证
export const mobileValidate = (data: object) => {
    return asyncRequest({
        url: "user/user/mobileValidate",
        method: "post",
        data
    });
};
// 新手机获得验证码
export const sendMobileCodeByMobileMobile = (data: object) => {
    return asyncRequest({
        url: "user/user/sendMobileCodeByModifyMobile",
        method: "post",
        data
    });
};
// 新手机验证码验证
export const mobileMobile = (data: object) => {
    return asyncRequest({
        url: "user/user/modifyMobile",
        method: "post",
        data
    });
};

// 修改密码获得验证码
export const sendMobileCodeByMobilePassword = (data: object) => {
    return asyncRequest({
        url: "user/user/sendMobileCodeByModifyPassword",
        method: "post",
        data
    });
};

export const checkModifyPasswordMobileCode = (data: object) => {
    return asyncRequest({
        url: "user/user/checkModifyPasswordMobileCode",
        method: "post",
        data
    });
};
//老邮箱获取验证码
export const sendEmailCodeByEmailValidate = (data: object) => {
    return request({
        url: "user/user/sendEmailCodeByEmailValidate",
        method: "post",
        data
    });
};
// 修改邮箱老邮箱验证码验证
export const emailValidate = (data: object) => {
    return request({
        url: "user/user/emailValidateNew",
        method: "post",
        data
    });
};
// 新邮箱获得验证码
export const sendEmailCodeByModifyEmail = (data: object) => {
    return request({
        url: "user/user/sendEmailCodeByModifyEmail",
        method: "post",
        data
    });
};
// 新邮箱验证码验证
export const emailEmailValidate = (data: object) => {
    return request({
        url: "user/user/modifyEmail",
        method: "post",
        data
    });
};