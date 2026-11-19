import { getSystemMode } from "@/api/common";

/**
 * 系统模式（我方新增，原版无此配置；biz_code=systemMode，后台设置下发）：
 * - all（默认，全功能）：商城 + 礼品卡全开，即现状；
 * - giftcard（礼品卡系统）：普通商城营销玩法入口收起；
 * - mall（商城系统）：礼品卡入口整体收起。
 *
 * app 启动时拉取一次存模块缓存（失败/未配置一律 all），页面渲染时同步读取做入口级门控；
 * 只隐藏入口，不拦直达链接、不改业务数据。
 */
export type SystemMode = "all" | "giftcard" | "mall";

let cachedMode: SystemMode = "all";
let fetched = false;
const listeners = new Set<(mode: SystemMode) => void>();

const normalize = (raw: unknown): SystemMode => {
    const mode = typeof raw === "string" ? raw : (raw as any)?.mode;
    return mode === "giftcard" || mode === "mall" ? mode : "all";
};

/** 当前系统模式（未拉取完成前为默认 all） */
export const currentSystemMode = (): SystemMode => cachedMode;

/** app 启动拉取系统模式并缓存；重复调用只发一次请求 */
export const fetchSystemMode = async (): Promise<SystemMode> => {
    if (fetched) return cachedMode;
    fetched = true;
    try {
        const result: any = await getSystemMode();
        cachedMode = normalize(result?.data ?? result);
    } catch {
        cachedMode = "all";
    }
    listeners.forEach((fn) => fn(cachedMode));
    return cachedMode;
};

/** 订阅模式就绪（页面首屏按 all 渲染、拉取完成后按真实模式重渲染时用） */
export const onSystemModeReady = (fn: (mode: SystemMode) => void): (() => void) => {
    listeners.add(fn);
    return () => {
        listeners.delete(fn);
    };
};
