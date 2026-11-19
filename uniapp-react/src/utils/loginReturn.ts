import Taro from "@tarojs/taro";

import { getTabbarPages } from "@/utils";

const home = "/pages/index/index";

export function loginReturnUrl(value: unknown): string {
    if (typeof value !== "string") return "";

    let url = value.trim();
    if (/^%2f/i.test(url)) {
        try {
            url = decodeURIComponent(url);
        } catch {
            return "";
        }
    }
    if (url.startsWith("/mobile/pages/")) url = url.slice("/mobile".length);
    if (url.startsWith("pages/")) url = `/${url}`;
    const path = url.split(/[?#]/)[0];
    if (!/^\/pages\/[\w/-]+$/.test(path) || path === "/pages/login/index" || path === "/pages/oauth/callback") {
        return "";
    }
    return url;
}

export function redirectAfterLogin(returnUrl?: unknown) {
    const url = loginReturnUrl(returnUrl) || loginReturnUrl(Taro.getStorageSync("URL")) || home;
    Taro.removeStorageSync("URL");
    const tabPage = getTabbarPages().includes(url.split(/[?#]/)[0]);
    const navigate = tabPage ? Taro.reLaunch({ url }) : Taro.redirectTo({ url });
    void navigate.catch(() => {
        if (url !== home) void Taro.reLaunch({ url: home }).catch(() => {});
    });
}
