import { useState } from "react";
import { View } from "@tarojs/components";

import BaseLogin from "@/pages/login/src/BaseLogin";
import "@/pages/login/index.scss";

/**
 * 登录页（pageType=5）整页预览分支（2026-11-19）。
 *
 * 与真实登录页（pages/login/index.tsx）同一渲染路径：该页本体就是 BaseLogin，
 * 装修的 mobileLogin module 只作 decorateConfig（颜色 + 边距）传入。真实页是
 * 先经 home/home/mobileLogin 接口取已发布配置再下发；预览改为直接把编辑器
 * 实时下发的 module 作 decorateConfig（未保存改动也能预览，口径同 shopId
 * 上下文下发先例），其余渲染与线上登录页完全一致。
 *
 * 交互安全：预览页（decoratePreview）已在 document 捕获层拦截 click/dblclick/
 * submit，登录按钮等点击不会真的提交；本组件也不拉 CSRF token（传空串，
 * BaseLogin 仅在真正发码/登录时才用，预览中不可达）。
 */
export default function LoginConfigPreview({ module }: { module: any }) {
    const [loginType, setLoginType] = useState("password");
    return (
        <View className="login-root-MjI4MnJhdHQ">
            <BaseLogin
                csrfToken=""
                onCsrfTokenChange={() => {}}
                loginType={loginType}
                onLoginTypeChange={setLoginType}
                decorateConfig={module ?? {}}
            />
        </View>
    );
}
