import type { ApiEnvelope } from "~/types/api";
import { ElMessage } from "element-plus";
import { aesEncrypt } from "~/components/verifition/utils/ase";
import { getCurrentRoute, formatArguments } from "~/utils/util";
import { useUserStore } from "~/store/user";
import { useRequestLocaleCode } from "~/composables/useRequestLocaleCode";

/**
 * API请求配置接口
 */
interface ApiConfig {
    url: string;
    method: "get" | "post" | "put" | "delete"; // 限制为具体的HTTP方法
    params?: any;
    data?: any;
    noSkipLogin?: boolean;
    headers?: Record<string, string>; // 可选的自定义请求头
    prefix?: string; // 自定义请求前缀，如 "/im/"
}

export const X_CLIENT_TYPE = "pc";
export const SILENT_LOGIN_REDIRECT_MESSAGE = "__NEXTRADE_SILENT_LOGIN_REDIRECT__";

/**
 * 401 静默登录重定向错误
 * - 业务侧若需要识别，统一通过 isSilentError(err) 判定
 * - 同时携带 silent / redirectLogin 字段，向后兼容旧的字段读取
 */
export class SilentLoginRedirectError extends Error {
    readonly silent = true;
    readonly redirectLogin = true;
    constructor() {
        super(SILENT_LOGIN_REDIRECT_MESSAGE);
        this.name = "SilentLoginRedirectError";
    }
}

/**
 * 判定一个错误是否为"无需 toast 的静默错误"
 * - 优先识别对象上的 silent 标记
 * - 兼容旧实现：识别 message / content 字段为静默魔串的情况
 */
export const isSilentError = (err: unknown): boolean => {
    if (!err || typeof err !== "object") return false;
    const candidate = err as { silent?: unknown; message?: unknown; content?: unknown };
    if (candidate.silent === true) return true;
    if (candidate.message === SILENT_LOGIN_REDIRECT_MESSAGE) return true;
    if (candidate.content === SILENT_LOGIN_REDIRECT_MESSAGE) return true;
    return false;
};

const { VITE_API_URL, VITE_API_PREFIX } = import.meta.env;
export const requestPrefix = VITE_API_URL + VITE_API_PREFIX;

/** 接口加密密钥（生产环境请修改） */
const key = "NexTradeApiSecret";

/**
 * 生成请求签名
 * @description 使用AES加密当前时间戳生成签名
 * @returns 包含Secret的对象
 */
const requestSecret = () => {
    const currentTimeStamp = Math.floor(Date.now() / 1000);
    const secret = aesEncrypt(currentTimeStamp, key);
    return {
        Secret: secret
    };
};

/**
 * 读取语言 Cookie（X-Locale-Code），兼容 Nuxt 实例上下文丢失的场景。
 * SSR 下 async 链路（await 之后）会脱离 Nuxt 实例上下文，此时直接 useCookie 会抛
 * "A composable that requires access to the Nuxt instance..."。降级策略：有上下文时走
 * useCookie（SSR 首个同步请求可读到请求级 Cookie）；上下文丢失时客户端从 document.cookie
 * 兜底，服务端无请求头可读则返回空串。
 */
const getLocaleCode = (): string => {
    if (tryUseNuxtApp()) {
        // 首次 SSR 中优先读取语言初始化写入的请求级状态；传入 Cookie 仅作为兼容回退。
        const requestLocaleCode = useRequestLocaleCode().value;
        if (requestLocaleCode) return requestLocaleCode;
        return useCookie("X-Locale-Code").value || "";
    }
    if (import.meta.client && typeof document !== "undefined") {
        const matched = document.cookie.match(/(?:^|;\s*)X-Locale-Code=([^;]*)/);
        return matched ? decodeURIComponent(matched[1]) : "";
    }
    return "";
};

/**
 * 生成请求头
 * @description 构建包含认证和客户端信息的请求头
 * @returns 请求头对象
 */
export const requestHeader = () => {
    const userStore = useUserStore();

    return {
        "X-Client-Type": X_CLIENT_TYPE,
        Authorization: "Bearer " + (userStore.isAuthenticated ? userStore.token : null),
        ...requestSecret(),
        "X-Locale-Code": getLocaleCode()
    };
};

/**
 * 构建请求配置
 * @description 根据API配置构建fetch请求所需的完整配置
 * @param config - API配置
 * @returns fetch请求配置
 */
const requestConfig = (config: ApiConfig) => {
    const data = formatArguments(config.data || config.params);

    const headers = {
        "Content-Type": "application/json",
        ...requestHeader()
    };

    if (config.headers) {
        Object.assign(headers, config.headers);
    }

    // 根据是否有自定义前缀来设置 baseURL
    const baseURL = config.prefix ? VITE_API_URL + config.prefix : requestPrefix + "/";

    // 设置请求头
    return {
        body: config.method === "post" ? JSON.stringify(config.data) : null,
        params: config.method === "post" ? "" : data,
        headers,
        baseURL,
        method: config.method,
        noSkipLogin: config.noSkipLogin || false
    };
};

/**
 * 跳转到登录页
 * @description 清除token并跳转到登录页，携带当前页面作为返回地址
 * @returns 返回Promise.reject
 */
export const toLogin = () => {
    const router = getCurrentRoute();
    useUserStore().clearToken();
    const loginPath = `/member/login?returnUrl=${router.fullPath ? router.fullPath : ""}`;
    // navigateTo(`/member/login?returnUrl=${router.path ? router.path : ""}`);
    navigateTo(loginPath);
    return Promise.reject(new SilentLoginRedirectError());
};

/** 默认错误数据 */
let errdata = {
    errcode: 1,
    message: "服务器连接失败，请检查网络"
};

/**
 * SSR请求函数
 * @description 使用useFetch进行服务端渲染兼容的请求
 * @template T - 响应数据类型
 * @param config - API配置
 * @returns Promise包装的响应数据
 */
export async function request<T>(config: ApiConfig): Promise<T> {
    const userStore = useUserStore();
    try {
        const result = await useFetch<ApiEnvelope<T>>(config.url, {
            ...requestConfig(config)
        });
        if (result.error.value) {
            // 检查错误代码，并根据需要处理
            return Promise.reject((result.error.value.data as ApiEnvelope<unknown>).data);
        }
        if (result.data.value) {
            if (result.data.value.code === 0) {
                const responseData = result.data.value.data;
                return responseData;
            } else {
                if (result.data.value.code === 401 && (userStore.token || !config.noSkipLogin)) {
                    return toLogin();
                }
                return Promise.reject(result.data.value);
            }
        }

        return Promise.reject(errdata);
    } catch (error) {
        if (!isSilentError(error)) {
            console.error(error);
        }
        return Promise.reject(error);
    }
}

/**
 * 客户端异步请求函数
 * @description 使用$fetch进行纯客户端请求（不支持SSR）
 * @template T - 响应数据类型
 * @param config - API配置
 * @returns Promise包装的响应数据
 */
export async function asyncRequest<T>(config: ApiConfig): Promise<T> {
    const userStore = useUserStore();
    try {
        const options = {
            ...requestConfig(config),
            lazy: true,
            server: false,
            onResponseError: (e) => {
                if (e.response._data.code === 403) {
                    // 禁止访问、无效Token
                } else if (e.response._data.code === 401) {
                    //登录已过期
                } else if (e.response._data.code === 400) {
                    // 服务器错误
                    ElMessage.error(e.response._data.msg);
                } else if (e.response._data.code === 500) {
                    ElMessage.error(e.response._data.msg);
                }
            }
        } satisfies NonNullable<Parameters<typeof $fetch>[1]> & { lazy: boolean; server: boolean };
        let result = await $fetch<ApiEnvelope<T> | string>(config.url, options);

        if (typeof result === "string") result = JSON.parse(result) as ApiEnvelope<T>;

        if (result.code === 0) {
            const responseData = result.data;
            return responseData;
        }

        if (result.code === 401 && (userStore.token || !config.noSkipLogin)) {
            return toLogin();
        }
        return Promise.reject(result);
    } catch (error) {
        if (!isSilentError(error)) {
            console.error(error);
        }
        return Promise.reject(error);
    }
}

export default request;
