import { useEffect, useState } from "react";
import Taro, { useRouter } from "@tarojs/taro";
import { View } from "@tarojs/components";

import { csrfCreate } from "@/api/login/login";
import { getMobileLoginDecorate } from "@/api/home/home";
import BaseLogin from "./src/BaseLogin";
import { useConfigStore } from "@/store/config";
import { useUserStore } from "@/store/user";
import { usePageGuards } from "@/utils/pageMixin";
import { loginReturnUrl, redirectAfterLogin } from "@/utils/loginReturn";

import "./index.scss";

/**
 * 登录页面（自旧 pages/login/index.vue 移植）
 * - 整合多种登录方式的入口页面
 * - 初始化CSRF令牌
 * - 根据平台展示不同登录选项
 * - 拉取移动端登录页装修配置（decorateSn=mobileUserLogin），下发核心组件按兜底应用
 */
export default function Login() {
    usePageGuards();

    const router = useRouter();

    const [loginType, setLoginType] = useState("password");

    // 登录页装修配置（仅颜色 + 边距）；未配置 / 失败时为空对象 → 核心组件全程兜底、样式零变化
    // 数据来源：新模块化装修（home/home/mobileLogin，Decorate 页 H5+pageType=5），取 moduleList 中 type=mobileLogin 的 module
    const [decorateConfig, setDecorateConfig] = useState<Record<string, string>>({});

    const [csrfToken, setCsrfToken] = useState("");

    const fetchLoginDecorate = async (previewId?: string | number) => {
        try {
            const result = await getMobileLoginDecorate(previewId);
            const moduleList = Array.isArray(result?.moduleList) ? result.moduleList : [];
            const item = moduleList.find((m: any) => m.type === "mobileLogin") ?? moduleList[0];
            setDecorateConfig(item?.module || {});
        } catch (error) {
            setDecorateConfig({});
        }
    };

    // 对应旧 onLoad（useRouter.params + useEffect）
    useEffect(() => {
        const options = router.params;
        const returnUrl = loginReturnUrl(options?.url);
        if (Taro.getStorageSync("token") || useUserStore.getState().token) {
            redirectAfterLogin(returnUrl);
            return;
        }
        if (options?.loginType) setLoginType(options.loginType);
        if (returnUrl) Taro.setStorageSync("URL", returnUrl);
        void useConfigStore.getState().getBaseConfigData();
        getCsrfCreateData();
        // 后台"预览"按钮打开 /pages/login/index?previewId=xxx，带上则取草稿版装修
        fetchLoginDecorate(options?.previewId);
        // eslint-disable-next-line react-hooks/exhaustive-deps
    }, []);

    const getCsrfCreateData = async () => {
        try {
            const result = await csrfCreate();
            setCsrfToken(result);
        } catch (error) {
            console.error("获取CSRF Token失败:", error);
        }
    };

    return (
        <View className="login-root-MjI4MnJhdHQ">
            <BaseLogin
                csrfToken={csrfToken}
                onCsrfTokenChange={setCsrfToken}
                loginType={loginType}
                onLoginTypeChange={setLoginType}
                decorateConfig={decorateConfig}
            />
        </View>
    );
}
