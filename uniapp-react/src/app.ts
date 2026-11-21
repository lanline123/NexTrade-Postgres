import type { PropsWithChildren } from "react";
import Taro, { useLaunch, useDidShow } from "@tarojs/taro";

import { useConfigStore } from "@/store/config";
import { resolveMiniShop, miniShopSilentLogin } from "@/store/miniShop";
import { fetchSystemMode } from "@/utils/systemMode";
import { configureRequestRuntime } from "@/utils/requestRuntime";
import { handleLogin } from "@/utils/handleLogin";
import { useI18nStore } from "@/store/i18n";
import init from "@/utils/init";
import { isOverseas } from "@/utils";

import { installInlineRpxH5 } from "@/utils/inlineRpxH5";

import "./app.scss";
// 全局装修样式（.module-ad-con 布局等）+ iconfont，自旧 App.vue 的 @import 平移
import "@/static/css/style.css";

installInlineRpxH5();

/** 基础配置（含 isIdentity 价格门控开关）上次拉取时刻：App 回前台时
 * 距上次超过 60 秒才重拉，避免频繁切前台反复请求。 */
let lastBaseConfigAt = 0;

function App({ children }: PropsWithChildren<any>) {
    useLaunch(() => {
        lastBaseConfigAt = Date.now();
        // 店铺独立绑定小程序：先解析本小程序作用域（店铺模式后续请求
        // 全带 X-Mini-App-Id 头），解析完成后在店铺模式下尝试静默登录
        void resolveMiniShop().then(() => miniShopSilentLogin());

        // 请求运行时：clientType / locale / 401 登录 / 启动初始化
        configureRequestRuntime({
            getClientType: () => useConfigStore.getState().XClientType,
            getLocaleCode: () => (isOverseas() ? useI18nStore.getState().langCode || "" : ""),
            handleLogin,
            initialize: init
        });

        // 设备信息与运行环境（对应旧 App.vue onLaunch 中的系统信息采集）
        const configStore = useConfigStore.getState();
        configStore.getWindowInfo();
        configStore.getSystemInfo();
        configStore.getMenuButtonInfo();

        // 启动初始化：i18n / 基础配置 / tabbar / 主题 / 用户 / 货币
        void init().then(() => {
            // App 级预取首页装修数据（原版 App.vue onLaunch 口径：init 完成后取数，
            // 与首页挂载后的请求经 store 在途去重——fetchHomeDecorateData 的
            // pendingHomeDecorateRequests——复用同一个请求，首页不再等页面挂载才发出）
            if (process.env.TARO_ENV === "h5") {
                // 先从 URL 解析 previewId 写入 store，确保预取能携带预览参数（原版同款处理）
                try {
                    const previewId = Number(new URLSearchParams(window.location.search).get("previewId")) || 0;
                    if (previewId > 0) {
                        useConfigStore.setState({ previewId });
                    }
                } catch {
                    // URL 解析失败不阻塞预取，按无预览口径继续
                }
            }
            const { fetchHomeDecorateData, previewId } = useConfigStore.getState();
            fetchHomeDecorateData(previewId).catch(() => undefined);
        });

        // 系统模式（我方新增）：启动拉取一次存全局缓存，供各页入口级门控同步读取
        void fetchSystemMode();
    });

    // 基础配置回前台刷新（2026-11-21 修复）：此前配置只在启动时拉一次，
    // 后台切换 isIdentity 等开关后，已打开的会话在重启前永远读旧值
    // （关掉实名看价后仍显示「认证后查看价格」）。回前台且距上次拉取
    // 超过 60 秒时重拉基础配置（getBaseConfigData 自带在途去重，
    // 与启动拉取并发时自动合并为同一请求）。
    useDidShow(() => {
        const now = Date.now();
        if (now - lastBaseConfigAt < 60_000) return;
        lastBaseConfigAt = now;
        void useConfigStore
            .getState()
            .getBaseConfigData()
            .catch(() => undefined);
    });

    // children 即当前渲染的页面
    return children;
}

export default App;
