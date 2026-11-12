import DOMPurify from "dompurify";

// 下划线转大驼峰
export function toPascalCase(str: string): string {
    const words = str.split("_").map((word) => word.charAt(0).toUpperCase() + word.slice(1));
    return words.join("");
}
// 首字母转大写
export function capitalize(str: string): string {
    return str.charAt(0).toUpperCase() + str.slice(1);
}

//校验是否有token
export function hasToken() {
    const token = useCookie("accessToken");
    if (token.value) {
        return true;
    } else {
        navigateTo("/member/login");
    }
}
// 获取当前路由
export function getCurrentRoute() {
    const router = useRouter();
    return router.currentRoute.value;
}
//是否多商户版
export const isMerchant = () => {
    return import.meta.env.VITE_IS_MERCHANT == 1;
};
export const isPro = () => {
    return import.meta.env.VITE_IS_PRO == 1;
};
export const isB2B = () => {
    return import.meta.env.VITE_IS_B2B == 1;
};
export const isOverseas = () => {
    return import.meta.env.VITE_IS_OVERSEAS == 1;
};
export const isDemo = () => {
    return import.meta.env.VITE_IS_DEMO == 1;
};
export const isStore = () => {
    return import.meta.env.VITE_IS_STORE == 1;
};
export const isO2o = () => isStore();

//是否多商户（shop）版
export const isShop = () => isMerchant();

//是否S2B2C(供应商)版
export function isS2b2c() {
    return import.meta.env.VITE_IS_VENDOR == 1;
}

// 是否移动端浏览器
export const isMobileBrowser = () => {
    if (typeof window === "undefined") {
        return false;
    }
    const ua = navigator.userAgent.toLowerCase();
    const mobilePattern =
        /android|webos|iphone|ipad|ipod|blackberry|iemobile|opera mini|mobile|phone|harmonyos|openharmony|hm os|arkweb/i;
    const isWechatBrowser = /micromessenger/.test(ua);
    const hasTouchCapability = "ontouchstart" in window || navigator.maxTouchPoints > 1;
    return mobilePattern.test(ua) || (isWechatBrowser && hasTouchCapability);
};
// 是否桌面端浏览器
export const isDesktopBrowser = () => {
    return !isMobileBrowser();
};

export const moneyFormat = (money: string): string => {
    return isNaN(parseFloat(money)) ? money : parseFloat(money).toFixed(2);
};
/**
 *
 * @param data
 * @returns  {any}
 */
export const formatArguments = (data: any) => {
    const result: any = {};
    if (typeof data === "object") {
        const dataKeys = Object.keys(data);
        for (const key of dataKeys) {
            const value = data[key];
            // 检查值是否为 null 或 undefined
            if (value === null || value === undefined || value === "") {
                continue;
            }
            if (typeof value === "object") {
                // 如果值是数组，检查其长度是否为 0
                if (Array.isArray(value) && value.length === 0) {
                    continue;
                } else if (Object.keys(value).length === 0) {
                    continue;
                }
            }
            result[key] = value;
        }
    }
    return result;
};

/**
 * 复制文本到剪贴板
 * @param text 要复制的文本
 * @param callback 复制成功后的回调函数
 */
export const copyText = (text: string, callback?: () => void, errorCallback?: () => void): void => {
    const textarea = document.createElement("textarea");
    textarea.value = text;
    textarea.style.position = "fixed";
    textarea.style.opacity = "0";
    document.body.appendChild(textarea);
    textarea.select();
    const success = document.execCommand("copy");
    document.body.removeChild(textarea);
    if (success) {
        if (callback) {
            callback();
        }
    } else {
        if (errorCallback) {
            errorCallback();
        }
    }
};

/**
 * 判断是否是url
 * @param str
 * @returns
 */
export function isUrl(str?: string | null): boolean {
    return typeof str === "string" && (str.indexOf("http://") != -1 || str.indexOf("https://") != -1);
}

// 安全打开链接：仅放行 http(s) 与站内相对路径，拦截 javascript:/data: 等危险协议，并强制 noopener 防止反向控制
export const safeOpen = (url?: string | null, target = "_blank", features?: string) => {
    if (!url || typeof window === "undefined") return;
    const trimmed = String(url).trim();
    const isRelative = trimmed.startsWith("/") || trimmed.startsWith("./") || trimmed.startsWith("../");
    if (!isRelative) {
        try {
            const protocol = new URL(trimmed, window.location.origin).protocol;
            if (protocol !== "http:" && protocol !== "https:") return;
        } catch {
            return;
        }
    }
    const opts = features ? features : undefined;
    const win = window.open(trimmed, target, opts);
    if (win) win.opener = null;
};


export const formatRichText = (html: string) => {
    if (!html) return "";
    let newContent = html.replace(/<img[^>]*>/gi, function (match, capture) {
        match = match.replace(/style="[^"]+"/gi, "").replace(/style='[^']+'/gi, "");
        match = match.replace(/width="[^"]+"/gi, "").replace(/width='[^']+'/gi, "");
        match = match.replace(/height="[^"]+"/gi, "").replace(/height='[^']+'/gi, "");
        return match;
    });
    newContent = newContent.replace(/style="[^"]+"/gi, function (match, capture) {
        match = match.replace(/width:[^;]+;/gi, "max-width:100%;").replace(/width:[^;]+;/gi, "max-width:100%;");
        return match;
    });
    newContent = newContent.replace(/<br[^>]*\/>/gi, "");
    newContent = newContent.replace(/\<img/gi, '<img style="max-width: 100%;height: auto;display: block;margin-top: 0;margin-bottom: 0;"');
    // 与标准版 PC 相同：仅在客户端净化（富文本在 ClientOnly 内渲染）。DOMPurify 移除 script、on* 事件和 javascript: 等危险协议。
    if (typeof window === "undefined") return newContent;
    return DOMPurify.sanitize(newContent, {
        ADD_TAGS: ["iframe"],
        ADD_ATTR: ["allow", "allowfullscreen", "frameborder", "scrolling", "target"]
    });
};


export const deepClone = (obj: any): any => {
    // 处理 null、undefined 或非对象类型
    if (obj === null || typeof obj !== "object") {
        return obj;
    }
    // 处理 Date 类型
    if (obj instanceof Date) {
        return new Date(obj.getTime());
    }
    // 处理 Array 类型
    if (Array.isArray(obj)) {
        return obj.map(item => deepClone(item));
    }
    // 处理 Object 类型
    if (typeof obj === "object") {
        const clonedObj: any = {};
        for (const key in obj) {
            if (Object.prototype.hasOwnProperty.call(obj, key)) {
                clonedObj[key] = deepClone(obj[key]);
            }
        }
        return clonedObj;
    }

    return obj;
};

/**
 * 静态资源输出
 * @param path
 * @returns
 */
export function staticResource(path: string, type = "images"): string {
    const staticResourcePath = import.meta.env.VITE_STATIC_RESOURCE || "static/mini";
    if (isUrl(path)) {
        return path;
    } else if (isUrl(staticResourcePath)) {
        return `${staticResourcePath}/${type}/${path}`;
    } else {
        return `${import.meta.env.VITE_API_URL || ""}/${staticResourcePath}/${type}/${path}`;
    }
}
