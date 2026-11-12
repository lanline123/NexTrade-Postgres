const AMAP_VERSION = "2.0";
/** 高德安全代理路径，演示站由 Nginx 在服务端追加 jscode */
const AMAP_SERVICE_PATH = "/_AMapService";
const TIANDITU_VERSION = "4.0";
const TIANDITU_SDK_URL = "https://api.tianditu.gov.cn/api";
/** 天地图脚本 onload 后仍可能在异步补齐命名空间，轮询等待上限 */
const TIANDITU_READY_TIMEOUT = 15000;
const TIANDITU_READY_INTERVAL = 50;

let tiandituPending: Promise<any> | null = null;

/**
 * 设置高德安全配置
 * @description 有安全密钥时沿用 securityJsCode；为空（演示站不下发）时改走本站 /_AMapService 代理。
 * 两者只设其一，同时存在时高德优先用 securityJsCode
 * @param jsSecret - 高德 JS API Secret（明文），可能为 null / 空串 / 缺失
 */
const setupAmapSecurity = (jsSecret?: string | null): void => {
    (window as any)._AMapSecurityConfig = jsSecret ? { securityJsCode: jsSecret } : { serviceHost: `${window.location.origin}${AMAP_SERVICE_PATH}` };
};

/**
 * 加载高德 JS SDK
 * @description AMapLoader 动态 import，保证 SSR 期不触碰浏览器 API
 * @param options.jsKey - 高德 JS API Key（明文）
 * @param options.jsSecret - 高德 JS API Secret（明文），为空时走安全代理
 * @param options.plugins - 需要的插件列表
 * @returns AMap 命名空间
 */
export const loadAmapSdk = async (options: { jsKey?: string; jsSecret?: string | null; plugins?: string[] }): Promise<any> => {
    const { jsKey, jsSecret, plugins = [] } = options;
    if (!jsKey) {
        throw new Error("未配置高德地图 JS Key，请先在后台地图设置中填写");
    }
    const AMapLoader = (await import("@amap/amap-jsapi-loader")).default;
    setupAmapSecurity(jsSecret);
    return AMapLoader.load({ key: jsKey, version: AMAP_VERSION, plugins });
};

/** 轮询等待天地图命名空间就绪 */
const waitForTiandituNamespace = (): Promise<any> => {
    return new Promise((resolve, reject) => {
        const startedAt = Date.now();
        const tick = () => {
            const T = (window as any).T;
            if (T && T.Map) {
                resolve(T);
                return;
            }
            if (Date.now() - startedAt >= TIANDITU_READY_TIMEOUT) {
                reject(new Error("天地图 SDK 加载超时，请检查网络或 tk 是否有效"));
                return;
            }
            window.setTimeout(tick, TIANDITU_READY_INTERVAL);
        };
        tick();
    });
};

/**
 * 加载天地图 JS SDK
 * @description 天地图无官方 loader 包，动态注入 script 并做并发去重；失败时清空缓存允许重试
 * @param tk - 天地图 JS API tk（明文，由 initConfigSettings 下发）
 * @returns T 命名空间
 */
export const loadTiandituSdk = (tk?: string): Promise<any> => {
    if (typeof window === "undefined") {
        return Promise.reject(new Error("天地图 SDK 仅能在客户端加载"));
    }
    const existing = (window as any).T;
    if (existing && existing.Map) return Promise.resolve(existing);
    if (tiandituPending) return tiandituPending;
    if (!tk) {
        return Promise.reject(new Error("未配置天地图 JS tk，请先在后台地图设置中填写"));
    }

    tiandituPending = new Promise((resolve, reject) => {
        const script = document.createElement("script");
        script.type = "text/javascript";
        script.async = true;
        script.src = `${TIANDITU_SDK_URL}?v=${TIANDITU_VERSION}&tk=${encodeURIComponent(tk)}`;
        script.onload = () => {
            waitForTiandituNamespace().then(resolve, reject);
        };
        script.onerror = () => {
            reject(new Error("天地图 SDK 加载失败，请检查网络连接"));
        };
        document.head.appendChild(script);
    }).catch((error) => {
        tiandituPending = null;
        throw error;
    });

    return tiandituPending;
};
