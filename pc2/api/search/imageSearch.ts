import { requestHeader, requestPrefix, toLogin } from "~/utils/request";
import { formatArguments } from "~/utils/util";
import { useUserStore } from "~/store/user";
import { useImageSearchStore } from "~/store/imageSearch";
import type { SearchFilterResult, SearchProductListResult } from "@/types/search/search.d";

/** 图搜能力与各端开关 */
export interface ImageSearchCapabilities {
    enabled: boolean;
    h5Enabled: boolean;
    pcEnabled: boolean;
    appEnabled: boolean;
    /** 店铺/门店内图搜开关，已结合总开关计算；端开关仍需另外判断 */
    shopEnabled?: boolean;
    formats: string[];
}

/** 上传成功后返回的图搜会话 */
export interface ImageSearchSession {
    searchToken: string;
}

/** 图搜请求错误：code 为 -1 表示用户取消，-2 表示超时，0 表示网络错误或响应不是约定的 JSON */
export interface ImageSearchError {
    code: number;
    message: string;
}

export const IMAGE_SEARCH_ABORTED = -1;
export const IMAGE_SEARCH_TIMEOUT = -2;
/** 图搜编号无效或已过期：需要用原图重新上传检索，其余失败不属于这种情况 */
export const IMAGE_SEARCH_TOKEN_EXPIRED = 40001;
/** 后台已关闭图搜 */
const IMAGE_SEARCH_DISABLED = 403;

/** 列表每页数量，上传与后续列表保持一致 */
export const IMAGE_SEARCH_PAGE_SIZE = 30;

// 上传包含图片预处理和云端检索，不能沿用普通列表的超时
const UPLOAD_TIMEOUT_MS = 90000;
const GUEST_SESSION_KEY = "imageSearchGuestSession";

/**
 * 游客图搜标识：上传、筛选、翻页必须是同一个值，生成一次后存在当前会话
 * 它不是登录凭证，已登录时后端以登录身份为准
 */
const getGuestSession = (): string => {
    // randomUUID 只在安全上下文可用，其余情况用随机字节生成同样长度的标识
    const create = () => {
        if (typeof crypto.randomUUID === "function") return crypto.randomUUID().replace(/-/g, "");
        return Array.from(crypto.getRandomValues(new Uint8Array(16)), (byte) => byte.toString(16).padStart(2, "0")).join("");
    };
    try {
        let session = sessionStorage.getItem(GUEST_SESSION_KEY) || "";
        if (!/^[A-Za-z0-9_-]{16,80}$/.test(session)) {
            session = create();
            sessionStorage.setItem(GUEST_SESSION_KEY, session);
        }
        return session;
    } catch {
        return create();
    }
};

const isImageSearchError = (error: unknown): error is ImageSearchError => {
    return !!error && typeof error === "object" && typeof (error as ImageSearchError).code === "number" && "message" in error;
};

interface ImageRequestOptions {
    method: "get" | "post";
    body?: FormData | Record<string, unknown>;
    params?: Record<string, unknown>;
    timeout?: number;
    signal?: AbortSignal;
}

/**
 * 请求失败统一出口：功能已关闭时同步关掉对应范围的开关，入口和结果页随之更新
 * 图搜请求都由结果页发起，按结果页当前的搜索范围判断关闭的是店铺开关还是本端开关
 */
const rejectImageRequest = (error: ImageSearchError): Promise<never> => {
    if (error.code === IMAGE_SEARCH_DISABLED) {
        const imageSearchStore = useImageSearchStore();
        imageSearchStore.markUnavailable(imageSearchStore.resultInShop);
    }
    return Promise.reject(error);
};

/**
 * 图搜专用请求（仅客户端）
 * 沿用项目请求头，游客额外携带图搜标识；失败统一整理成 { code, message }，保留接口返回的原因
 */
const imageRequest = async <T>(url: string, options: ImageRequestOptions): Promise<T> => {
    const userStore = useUserStore();
    const controller = new AbortController();
    let timedOut = false;
    const onAbort = () => controller.abort();
    options.signal?.addEventListener("abort", onAbort);
    const timer = options.timeout
        ? setTimeout(() => {
              timedOut = true;
              controller.abort();
          }, options.timeout)
        : null;

    let result: any;
    try {
        result = await $fetch(url, {
            baseURL: requestPrefix + "/",
            method: options.method,
            body: options.body,
            params: options.params,
            headers: {
                ...requestHeader(),
                ...(userStore.isAuthenticated ? {} : { "X-Image-Search-Session": getGuestSession() })
            },
            signal: controller.signal
        });
    } catch (error: any) {
        if (timedOut) return Promise.reject({ code: IMAGE_SEARCH_TIMEOUT, message: "" } as ImageSearchError);
        if (controller.signal.aborted) return Promise.reject({ code: IMAGE_SEARCH_ABORTED, message: "" } as ImageSearchError);
        const data = error?.data;
        const code = Number(data?.code ?? error?.statusCode ?? error?.status) || 0;
        // HTTP 状态返回的登录失效与业务码一致处理
        if (code === 401 && userStore.token) return toLogin();
        return rejectImageRequest({
            code,
            message: typeof data?.message === "string" ? data.message : ""
        });
    } finally {
        if (timer) clearTimeout(timer);
        options.signal?.removeEventListener("abort", onAbort);
    }

    if (typeof result === "string") {
        try {
            result = JSON.parse(result);
        } catch {
            return Promise.reject({ code: 0, message: "" } as ImageSearchError);
        }
    }
    if (result?.code === 0) return result.data as T;
    if (result?.code === 401 && userStore.token) return toLogin();
    return rejectImageRequest({ code: Number(result?.code) || 0, message: typeof result?.message === "string" ? result.message : "" });
};

/**
 * 把图搜错误转成可展示的文案
 * 优先使用接口返回的原因，缺失时按状态给出兜底提示
 * @param error - 图搜请求抛出的错误
 * @param t - 翻译函数，只用于兜底提示
 */
export const resolveImageSearchErrorMessage = (error: unknown, t: (key: string) => string): string => {
    if (!isImageSearchError(error)) return t("图片搜索失败，请重试");
    // 编号失效的接口文案面向开发者，统一用页面自己的说法
    if (error.code === IMAGE_SEARCH_TOKEN_EXPIRED) return t("本次图片搜索已失效，请重新搜索");
    if (error.message) return error.message;
    if (error.code === IMAGE_SEARCH_TIMEOUT || error.code === 504) return t("图片搜索超时，请重试");
    if (error.code === 429) return t("图片搜索繁忙，请稍后重试");
    if (error.code === 409 || error.code === 502) return t("图片搜索暂不可用");
    if (error.code === 413) return t("图片过大，请更换图片");
    if (error.code === 0) return t("网络异常，请稍后重试");
    return t("图片搜索失败，请重试");
};

export const isImageSearchAborted = (error: unknown): boolean => {
    return isImageSearchError(error) && error.code === IMAGE_SEARCH_ABORTED;
};

export const isImageSearchDisabled = (error: unknown): boolean => {
    return isImageSearchError(error) && error.code === IMAGE_SEARCH_DISABLED;
};

export const isImageSearchExpired = (error: unknown): boolean => {
    return isImageSearchError(error) && error.code === IMAGE_SEARCH_TOKEN_EXPIRED;
};

/**
 * 读取图搜能力和各端开关
 * @returns 端开关已结合总开关计算；图搜默认关闭，由后台开关控制
 */
export const getImageSearchCapabilities = () => {
    return imageRequest<ImageSearchCapabilities>("search/image/capabilities", { method: "get" });
};

/**
 * 上传图片并创建图搜会话
 * @param file - 待搜索的图片文件
 * @param signal - 取消信号；取消只中断本次请求，不保证撤销后端已发出的云端检索
 * @param shopId - 店铺内搜索时传店铺 ID，省略为全站；后续列表和筛选沿用上传时的店铺，不再单独传
 * @returns 图搜会话
 */
export const searchByImage = (file: File, signal?: AbortSignal, shopId?: number) => {
    const form = new FormData();
    form.append("file", file);
    form.append("client", "pc");
    if (shopId !== undefined) form.append("shopId", String(shopId));
    form.append("size", String(IMAGE_SEARCH_PAGE_SIZE));
    return imageRequest<ImageSearchSession>("search/image/search", {
        method: "post",
        body: form,
        timeout: UPLOAD_TIMEOUT_MS,
        signal
    });
};

/**
 * 图搜结果的商品列表，条件只在本次图片召回的商品内生效
 * @param data - 含 searchToken 的查询条件、排序与分页
 * @returns 商品分页
 */
export const getImageSearchProductList = (data: Record<string, unknown>) => {
    return imageRequest<SearchProductListResult>("search/search/getProduct", { method: "post", body: data });
};

/**
 * 图搜结果的筛选项
 * @param data - 与商品列表一致的查询条件
 * @returns 本次相似商品内的分类、品牌和最高价
 */
export const getImageSearchFilter = (data: Record<string, unknown>) => {
    return imageRequest<SearchFilterResult>("search/search/getFilter", { method: "post", body: data });
};

/**
 * 图搜结果的属性筛选建议
 * @param data - 与商品列表一致的查询条件
 * @returns 属性名称及可选值
 */
export const getImageSearchAttributes = (data: Record<string, unknown>) => {
    const { attrs, ...rest } = data as { attrs?: Array<{ attrName: string; attrValue: string[] }> } & Record<string, unknown>;
    const params: Record<string, unknown> = formatArguments(rest);
    // GET 的对象数组必须按下标展开，后端不接受 JSON 字符串
    (attrs ?? []).forEach((attr, i) => {
        params[`attrs[${i}].attrName`] = attr.attrName;
        (attr.attrValue ?? []).forEach((value, j) => {
            params[`attrs[${i}].attrValue[${j}]`] = value;
        });
    });
    return imageRequest<any>("search/search/getAttributeSuggestions", { method: "get", params });
};
