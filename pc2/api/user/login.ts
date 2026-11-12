import { request, asyncRequest } from "@/utils/request";
import type { quickLoginSettingResult, wechatEventResult, wechatLoginUrlResult } from "~/types/user/login";

/**
 * 用户登录
 * @description 账号密码或手机验证码登录
 * @param data - 登录参数（账号、密码/验证码等）
 * @param headers - 自定义请求头
 * @returns 登录结果，包含token和用户信息
 */
export const userSignin = (data: any, headers = {}) => {
    return asyncRequest<any>({
        url: "user/login/signin",
        method: "post",
        data,
        headers
    });
};

/**
 * 创建CSRF令牌
 * @description 获取CSRF令牌用于表单安全验证
 * @returns CSRF令牌
 */
export const csrfCreate = () => {
    return asyncRequest({
        url: "common/csrf/create",
        method: "get"
    });
};

/**
 * 发送手机验证码
 * @description 发送短信验证码（登录/注册/找回密码）
 * @param data - 手机号及用途类型
 * @returns 发送结果
 */
export const sendMobileCode = (data: any) => {
    return asyncRequest({
        url: "user/login/sendMobileCode",
        method: "post",
        data
    });
};
/**
 * 发送邮箱验证码
 * @description 发送邮件验证码（找回密码等场景）
 * @param data - 邮箱地址及用途类型
 * @returns 发送结果
 */
export const sendEmailCode = (data: any) => {
    return asyncRequest({
        url: "user/login/sendEmailCode",
        method: "post",
        data
    });
};

/**
 * 验证邮箱
 * @description 验证邮箱地址是否存在/有效
 * @param data - 邮箱地址
 * @returns 验证结果
 */
export const checkEmail = (data: any) => {
    return request({
        url: "user/login/checkEmail",
        method: "post",
        data
    });
};

/**
 * 获取快捷登录配置
 * @description 获取启用的第三方登录方式（微信、QQ等）
 * @returns 快捷登录配置
 */
export const getQuickLoginSetting = () => {
    return asyncRequest<quickLoginSettingResult>({
        url: "user/login/getQuickLoginSetting",
        method: "get"
    });
};

/**
 * 获取微信登录URL
 * @description 获取微信扫码登录的二维码地址
 * @returns 微信登录URL
 */
export const getWechatLoginUrl = () => {
    return asyncRequest<wechatLoginUrlResult>({
        url: "user/login/getWxLoginUrl",
        method: "get"
    });
};

/**
 * 检查微信扫码事件
 * @description 轮询检查用户是否已扫码确认登录
 * @param data - 轮询参数
 * @returns 扫码状态和登录结果
 */
export const checkWechatEvent = (data: any) => {
    return asyncRequest<wechatEventResult>({
        url: "user/login/wechatEvent",
        method: "post",
        data
    });
};

/**
 * 验证手机号
 * @description 验证手机号是否已注册
 * @param data - 手机号
 * @returns 验证结果
 */
export const checkMobile = (data: any) => {
    return asyncRequest({
        url: "user/login/checkMobile",
        method: "post",
        data
    });
};

/**
 * 找回密码
 * @description 通过验证码重置密码
 * @param data - 验证信息和新密码
 * @returns 重置结果
 */
export const forgetPassword = (data: any) => {
    return asyncRequest({
        url: "user/login/forgetPassword",
        method: "post",
        data
    });
};

/**
 * 获取第三方OAuth登录URL
 * @description 获取第三方平台的授权登录地址
 * @param path - 第三方平台标识（如wechat、qq）
 * @param redirectUri - 授权回调地址
 * @returns 授权URL
 */
export const getOauth = (path: string, redirectUri: string) => {
    return asyncRequest<string>({
        url: `user/oauth/render/${path}?redirectUri=${redirectUri}`,
        method: "get"
    });
};

/**
 * 第三方登录回调处理
 * @description 处理OAuth授权回调，完成登录绑定
 * @param path - 第三方平台标识
 * @param redirectUri - 回调地址
 * @param data - 授权回调参数（code等）
 * @returns 登录结果
 */
export const getOauthCallback = (path: string, redirectUri: string, data: object) => {
    return asyncRequest<any>({
        url: `user/oauth/callback/${path}?redirectUri=${redirectUri}`,
        method: "post",
        data
    });
};

/**
 * 用户退出登录
 * @description 清除登录状态
 * @returns 退出结果
 */
export const userLogout = () => {
    return asyncRequest({
        url: "user/user/logout",
        method: "post"
    });
};
