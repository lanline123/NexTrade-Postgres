import { useEffect, useRef } from "react";
import { View } from "@tarojs/components";
import Taro, { useRouter } from "@tarojs/taro";

import { getOauthCallback, getOauthInfo } from "@/api/login/login";
import { useCartStore } from "@/store/cart";
import { useUserStore } from "@/store/user";
import { t } from "@/utils/i18n";
import { isWeixinBrowser } from "@/utils";
import { redirectAfterLogin } from "@/utils/loginReturn";
import { usePageGuards } from "@/utils/pageMixin";

import "./callback.scss";

export default function OauthCallback() {
    usePageGuards();
    const { params } = useRouter();
    const started = useRef(false);

    useEffect(() => {
        if (started.current) return;
        started.current = true;

        const finishAuthorization = async () => {
            try {
                // Taro H5 retains percent escapes in router params. OAuth expects the original values.
                const query = process.env.TARO_ENV === "h5" ? new URLSearchParams(window.location.search) : undefined;
                const code = query?.get("code") ?? params.code;
                const state = query?.get("state") ?? params.state;
                const provider = query?.get("provider") ?? params.provider;
                if (!code || !state) throw new Error(t("缺少授权参数"));

                let result: { type?: number; token?: string; openData?: unknown; returnUrl?: string };
                if (provider === "wechat") {
                    const expectedState = Taro.getStorageSync("wechatLoginState");
                    Taro.removeStorageSync("wechatLoginState");
                    if (!isWeixinBrowser() || expectedState !== state) throw new Error(t("授权已过期，请重新登录"));

                    result = await getOauthInfo({ code });
                    if (result.type === 2 && result.openData) {
                        Taro.setStorageSync("openData", result.openData);
                        await Taro.redirectTo({ url: "/pages/user/bindMobilePhone/index" });
                        return;
                    }
                } else {
                    const source = state.includes(":") ? state.split(":")[0] : Taro.getStorageSync("oauthPath");
                    Taro.removeStorageSync("oauthPath");
                    if (!["google", "facebook", "twitter"].includes(source)) throw new Error(t("缺少授权方式"));
                    result = await getOauthCallback(source, { code, state });
                }
                if (!result.token) throw new Error(t("授权失败"));

                const userStore = useUserStore.getState();
                userStore.setToken(result.token);
                void userStore.getUserInfo();
                void useCartStore.getState().fetchCartCount(true);
                void Taro.showToast({ title: t("登录成功"), icon: "success" });
                redirectAfterLogin(result.returnUrl);
            } catch (error: any) {
                Taro.removeStorageSync("oauthPath");
                Taro.removeStorageSync("wechatLoginState");
                void Taro.showToast({ title: error?.message || t("授权失败"), icon: "none" });
                await Taro.redirectTo({ url: "/pages/login/index" }).catch(() => {});
            }
        };

        void finishAuthorization();
    }, [params]);

    return <View className="oauth-callback" />;
}
