import Taro from "@tarojs/taro";
import { create } from "zustand";

/**
 * 店铺独立绑定小程序（本平台新增，论坛帖 487946，模式①）客户端作用域。
 *
 * 每个店铺的小程序是独立构建：运行时用 `getAccountInfoSync` 读自身
 * appid，上报给服务端（请求头 `X-Mini-App-Id` + 启动解析接口），
 * 服务端按绑定表把该 appid 的全部请求强制收口到对应店铺（未登记
 * appid 直接 403）。平台小程序 appid 解析为 platform 模式，行为与
 * 原来完全一致；H5 / App 无 appid，恒为平台模式。
 */

export type MiniShopMode = "" | "platform" | "shop";

interface MiniShopState {
    /** 本小程序自身 appid（非微信小程序环境为空串） */
    appId: string;
    /** 作用域模式：未解析 "" / 平台 "platform" / 店铺 "shop" */
    mode: MiniShopMode;
    /** 店铺模式下绑定的店铺 ID（平台模式为 0） */
    shopId: number;
    /** 店铺模式下绑定的店铺名 */
    shopTitle: string;
    /** 启动解析是否已完成（首页重定向等决策等它落定） */
    resolved: boolean;
}

export const useMiniShopStore = create<MiniShopState>(() => ({
    appId: "",
    mode: "",
    shopId: 0,
    shopTitle: "",
    resolved: false
}));

/** 读取当前小程序自身 appid（仅微信小程序运行时有值，其余环境回空串）。 */
export function getOwnAppId(): string {
    if (process.env.TARO_ENV !== "weapp") return "";
    try {
        const info = Taro.getAccountInfoSync();
        return info?.miniProgram?.appId || "";
    } catch {
        return "";
    }
}

/** 请求头/登录参数用：已知的自身 appid（同步读 store，未解析时现读运行时）。 */
export function currentMiniAppId(): string {
    return useMiniShopStore.getState().appId || getOwnAppId();
}

/**
 * 启动解析：向服务端询问本 appid 的作用域（平台/店铺）。
 * 失败时按平台模式兜底放行（店铺构建若被 403 说明未绑定，服务端会在
 * 每个业务请求上硬拒绝，这里不重复拦截、不阻塞启动）。
 */
export async function resolveMiniShop(): Promise<void> {
    const appId = getOwnAppId();
    if (!appId) {
        useMiniShopStore.setState({ mode: "platform", resolved: true });
        return;
    }
    useMiniShopStore.setState({ appId });
    try {
        const { default: request } = await import("@/utils/request");
        const ctx = await request<any>({
            url: "shop/miniProgram/context",
            method: "get",
            params: { appId }
        });
        if (ctx?.mode === "shop" && Number(ctx.shopId) > 0) {
            useMiniShopStore.setState({
                mode: "shop",
                shopId: Number(ctx.shopId),
                shopTitle: String(ctx.shopTitle || ""),
                resolved: true
            });
            return;
        }
        useMiniShopStore.setState({ mode: "platform", shopId: 0, shopTitle: "", resolved: true });
    } catch {
        useMiniShopStore.setState({ mode: "platform", resolved: true });
    }
}

/**
 * 店铺模式静默登录：`wx.login` code → 服务端按 appid 换 openid，
 * 已授权（本店小程序维度）直接登录；未知 openid 回 bindTicket 存入
 * openData，等用户走手机号登录时由绑手机链按手机号归一到同一账号
 * （店铺小程序与平台小程序分属不同开放平台账号，unionid 不互通）。
 * 已有 token / 非店铺模式 / 非微信环境直接跳过；失败静默不阻塞。
 */
export async function miniShopSilentLogin(): Promise<void> {
    if (process.env.TARO_ENV !== "weapp") return;
    if (useMiniShopStore.getState().mode !== "shop") return;
    if (Taro.getStorageSync("token")) return;
    try {
        const loginRes = await Taro.login();
        if (!loginRes?.code) return;
        const { getMiniLoginInfoByCode } = await import("@/api/login/login");
        const result: any = await getMiniLoginInfoByCode({
            code: loginRes.code,
            appId: currentMiniAppId()
        });
        if (result?.type == 1 && result.token) {
            const { useUserStore } = await import("@/store/user");
            useUserStore.getState().setToken(result.token);
            await useUserStore.getState().getUserInfo();
        } else if (result?.type == 2 && result.openData) {
            Taro.setStorageSync("openData", result.openData);
        }
    } catch {
        // 静默登录失败不阻塞：用户可从「我的」走手机号登录（同归一链）
    }
}
