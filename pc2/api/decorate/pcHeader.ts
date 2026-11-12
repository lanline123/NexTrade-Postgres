import { request, asyncRequest } from "@/utils/request";
import type { ApiPcHeaderVO, ApiPcLoginDecorateVO } from "~/types/decoration/decorationClientVo";

/**
 * 装修 - PC 头部
 * 后端 home/home/pcHeader 取 PC 首页 pageModule.pcHeader 原样返回 { pcHeader: {...} }。
 * 用 request（useFetch，SSR 兼容），与 getNav/getBaseConfig 一致，全站统一加载。
 */
export const decorateApi = {
    getPcHeader: (params?: Record<string, unknown>) =>
        request<ApiPcHeaderVO>({
            url: "home/home/pcHeader",
            method: "get",
            params
        }),

    /**
     * 装修 - PC 头部（客户端重取版）
     * previewId 变化（演示站切风格 / 手动改 query）后在事件回调、watch 里重取时使用；
     * 这些时机已脱离 setup 上下文，useFetch 不可用，改走 $fetch。
     */
    getPcHeaderAsync: (params?: Record<string, unknown>) =>
        asyncRequest<ApiPcHeaderVO>({
            url: "home/home/pcHeader",
            method: "get",
            params
        }),

    /**
     * 装修 - PC 登录页
     * 后端 home/home/pcLogin 返回 DecorateModuleListVO，moduleList[0] 为 pcLogin 模块，
     * 携带 loginMode / backgroundPic / backgroundColor / headerHeight / headerShadow。
     * 用 request（useFetch，SSR 兼容），保证登录页首屏按配置直出，避免模式切换闪动。
     */
    getPcLogin: (params?: Record<string, unknown>) =>
        request<ApiPcLoginDecorateVO>({
            url: "home/home/pcLogin",
            method: "get",
            params,
            noSkipLogin: true
        })
};
