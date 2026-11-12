import { groupLinkUrl } from "./shop-group";
import { useCommonStore } from "@/store/common";
import { buildCategoryPath, buildProductPath, buildShopPath } from "@/utils/seo-url";

// 格式化图片地址
export function imageFormat(path: string | undefined) {
    const commonStore = useCommonStore();
    if (!path) {
        return "";
    }
    return path.includes("http") ? path : commonStore.storageUrl + path;
}

interface PicSource {
    picUrl?: string;
    picThumb?: string;
    picLarge?: string;
    picOriginal?: string;
}

interface SkuPicSource extends PicSource {
    skuData?: unknown[];
    originSkuData?: unknown[];
    sku?: {
        skuData?: unknown[];
    } | null;
}

// 按 picUrl > picThumb > picLarge 取图，不拼接域名（列表缩略等场景）
export function pickPic(source: PicSource | null | undefined): string {
    if (!source) {
        return "";
    }
    return source.picUrl || source.picThumb || source.picLarge || "";
}

/**
 * 订单商品优先展示所选规格图；多个规格维度都配置图片时，以最后一个有效规格图为准。
 * 数据源优先级：商品项顶层 skuData > originSkuData > sku.skuData。
 */
export function pickSkuPic(source: SkuPicSource | null | undefined): string {
    if (!source) {
        return "";
    }

    const skuData =
        Array.isArray(source.skuData) && source.skuData.length > 0
            ? source.skuData
            : Array.isArray(source.originSkuData) && source.originSkuData.length > 0
              ? source.originSkuData
              : Array.isArray(source.sku?.skuData)
                ? source.sku.skuData
                : [];
    for (let index = skuData.length - 1; index >= 0; index--) {
        const skuItem = skuData[index] as { attrPic?: string; attrPicThumb?: string } | undefined;
        const skuPic = skuItem?.attrPicThumb || skuItem?.attrPic;
        if (skuPic) {
            return skuPic;
        }
    }

    return pickPic(source);
}

// 详情主图/放大镜：优先大图
export function pickPicLarge(source: PicSource | null | undefined): string {
    if (!source) {
        return "";
    }
    return source.picLarge || source.picOriginal || source.picUrl || source.picThumb || "";
}

// 按 picUrl > picThumb > picLarge 取图并拼接 storageUrl 前缀
export function productImage(source: PicSource | null | undefined): string {
    return imageFormat(pickPic(source));
}

export function productSkuImage(source: SkuPicSource | null | undefined): string {
    return imageFormat(pickSkuPic(source));
}

// 详情主图/放大镜取图并拼接 storageUrl 前缀
export function productImageLarge(source: PicSource | null | undefined): string {
    return imageFormat(pickPicLarge(source));
}

export function imageFormatList(path: string[] | undefined) {
    if (path && path?.length > 0) {
        let temp: any = [];
        path?.forEach((item: any) => {
            temp.push(imageFormat(item.picUrl));
        });
        return temp;
    } else {
        return [];
    }
}

// 格式化金额
export function priceFormat(price: number | string, currencyFormat = true) {
    if (!price) {
        return "";
    }
    const commonStore = useCommonStore();
    const currency = commonStore.dollarSign ? "<span class='util'>" + commonStore.dollarSign + "</span>" : "<span class='util'>¥</span>";
    const num = typeof price === "string" ? parseFloat(price) : price;
    if (currencyFormat) {
        return currency + "<span class=decimal>" + num.toFixed(2) + "</span>";
    } else {
        return num.toFixed(2);
    }
}

/** 允许出现在装修链接里的协议，其余（javascript:/data:/vbscript: 等）一律拦掉 */
const SAFE_LINK_PROTOCOL = /^(https?|mailto|tel):$/i;

/**
 * 装修链接安全过滤：只放行 http(s)/mailto/tel 与站内相对路径。
 *
 * 装修数据里的 picLink/moreLink 由后台配置，会原样进 NuxtLink 的 :to。NuxtLink 判定为外链后
 * 渲染成裸 <a href>，点击时不拦截默认行为，`javascript:` 这类协议会直接执行，构成存储型 XSS。
 * 协议相对地址（//host）等价于外链，同样不放行。
 */
export function sanitizeLinkUrl(url: string | undefined | null): string {
    // 装修 picLink 常为 null/缺 path；resolveUrl 也可能回落 undefined，这里必须先挡空值
    if (url == null || typeof url !== "string") {
        return "";
    }
    // 浏览器会忽略 URL 中的空白与控制字符，先剔除再判协议，避免 "java\tscript:" 这类绕过
    const cleaned = url.replace(/[\u0000-\u0020\u00a0\u2028\u2029]/g, "");
    if (!cleaned || cleaned.startsWith("//")) {
        return "";
    }
    const scheme = cleaned.match(/^[a-z][a-z0-9+.-]*:/i);
    if (scheme && !SAFE_LINK_PROTOCOL.test(scheme[0])) {
        return "";
    }
    return url;
}

/** 可直接使用的链接形态：带协议、协议相对、站内绝对路径、锚点、查询串 */
const DIRECT_LINK = /^(?:[a-z][a-z0-9+.-]*:|\/\/|\/|#|\?)/i;

/**
 * 站内相对路径统一补前导 "/"。
 *
 * 装修链接（picLink.link、导航 link 等）语义上都是站内绝对地址，但后台存的值可能不带前导斜杠。
 * 这类值直接进 NuxtLink 的 :to 会被路由按"相对当前页"解析，在非首页点击时拼出错误地址，
 * 例如在 /member/coupon/list 点 "member/coupon/list" 会跳到 /member/coupon/member/coupon/list。
 */
function toInternalAbsolute(url: string): string {
    if (!url || DIRECT_LINK.test(url)) {
        return url;
    }
    return `/${url}`;
}

// 链接格式化
export function urlFormat(params: string | undefined | null | { path?: string; [key: string]: any }): string {
    // 补前导斜杠必须放在安全过滤之后：否则 "java\tscript:alert(1)" 这类含控制字符的串会因开头被改成 "/" 而躲过协议检测
    return toInternalAbsolute(sanitizeLinkUrl(resolveUrl(params)));
}

function resolveUrl(params: string | undefined | null | { path?: string; [key: string]: any }): string {
    if (params == null) {
        return "";
    } else if (typeof params === "string") {
        return params;
    } else {
        switch (params.path) {
            case "product":
                return buildProductPath({
                    seoLink: params.seoLink,
                    sn: params.sn ?? params.id,
                    skuId: params.skuId,
                    type: params.type,
                    grouponPromotionRecordId: params.grouponPromotionRecordId,
                    shopId: params.shopId
                });
            case "exchange":
                return "/exchange/" + params.id + "";
            case "list":
                return buildCategoryPath({
                    seoLink: params.seoLink,
                    id: params.id,
                    pId: params.pId
                });
            case "article":
                // 咨询页面
                return "/article/news/list?articleId=" + params.id;
            case "brand":
                // 品牌搜索
                return "/list?brand=" + params.id;
            case "coupon":
                // 优惠券详情
                return "/coupon/list/?id=" + params.id;
            case "shop":
                // 店铺页面
                return buildShopPath({
                    seoLink: params.seoLink,
                    id: params.id
                });
            case "shopGroup":
            case "storeGroup":
                return groupLinkUrl(params);
            case "store":
                // 咨询页面
                return "/store/?id=" + params.id;
            case "base": {
                // 基础页面：统一补前导 "/"，并去掉 link 自带的多余斜杠。
                // 首页配置常为 link:"/"，若直接拼接会得到 "//"，再被 sanitizeLinkUrl 当成协议相对地址拦成空串。
                const baseLink = String(params.link ?? "");
                if (!baseLink) return "";
                return `/${baseLink.replace(/^\/+/, "")}`;
            }
            case "order":
                return `/member/order/info?id=${params.id}`;
            case "custom": {
                // 自定义页面：外链原样返回；站内相对路径补前导 "/"（与 base 分支保持一致）。
                // 否则像 "join/intro"（缺前导斜杠）会被路由按“相对当前页”解析，拼出错误地址（如多出 search）。
                const customLink = String(params.link ?? "");
                if (!customLink) return "";
                // 必须按前缀判外链：isUrl 用的是 indexOf，"javascript:alert('http://')" 这类串也会被当成外链放行
                return /^https?:\/\//i.test(customLink) ? customLink : `/${customLink.replace(/^\/+/, "")}`;
            }
            case "storeInfo":
                return "/store/info?shopId=" + params.shopId;
            default:
                return typeof params.path === "string" ? params.path : "";
        }
    }
}

/**
 * 评分格式化：整数补一位小数，小数原样返回，空值返回 "0.0"
 * @param rating 评分
 * @returns 格式化后的评分字符串
 */
export function ratingFormat(rating: number | string | undefined | null): string {
    if (rating === undefined || rating === null || rating === 0) {
        return "0.0";
    }
    const num = typeof rating === "string" ? parseFloat(rating) : rating;
    if (isNaN(num)) {
        return "0.0";
    }
    // 检查是否为整数
    if (Number.isInteger(num)) {
        return num.toFixed(1);
    }
    // 如果是小数，直接返回
    return num.toString();
}

/**
 * 获取倒计时 返回格式xx天xx:xx:xx
 * @param endTime 结束时间戳(毫秒)
 * @param dayText “天”的文案，由调用方传入 $t("天")；设为必填以避免漏掉多语言
 * @returns 格式化后的倒计时字符串
 */
export function getCountdownFormat(endTime: number, dayText: string): string {
    const now = Date.now();
    const diff = endTime - now;

    if (diff <= 0) {
        return "";
    }

    const seconds = Math.floor(diff / 1000);
    const minutes = Math.floor(seconds / 60);
    const hours = Math.floor(minutes / 60);
    const days = Math.floor(hours / 24);

    const remainingSeconds = String(seconds % 60).padStart(2, "0");
    const remainingMinutes = String(minutes % 60).padStart(2, "0");
    const remainingHours = String(hours % 24).padStart(2, "0");
    if (days === 0) {
        return `${remainingHours}:${remainingMinutes}:${remainingSeconds}`;
    } else {
        return `${days}${dayText} ${remainingHours}:${remainingMinutes}:${remainingSeconds}`;
    }
}

/**
 * 隐藏用户名敏感信息
 * @param username 用户名
 * @returns 隐藏处理后的用户名
 */
export function hideUsername(username: string): string {
    if (!username) {
        return "";
    }

    const length = username.length;

    if (length === 1) {
        // 如果只有一个字符直接展示
        return username;
    } else if (length === 2) {
        // 如果两个字符隐藏最后一个字符
        return username.charAt(0) + "*";
    } else {
        // 大于等于三个字符的只保留第一个和最后一个字符，其余*号表示
        const firstChar = username.charAt(0);
        const lastChar = username.charAt(length - 1);
        const middleStars = "*".repeat(length - 2);
        return firstChar + middleStars + lastChar;
    }
}

/**
 * 格式化数字：有小数则原样输出，没有则补足指定位数的小数0
 * @param num 数字或字符串
 * @param decimalPlaces 小数位数，默认1
 */
export function formatPadZero(num: number | string, decimalPlaces = 1): string {
    const str = String(num);
    if (str.includes(".")) {
        // 有小数，去除多余的0（如5.00 -> 5.0），但不补0
        return parseFloat(str).toString();
    } else if (decimalPlaces > 0) {
        return `${str}.${"0".repeat(decimalPlaces)}`;
    } else {
        return str;
    }
}

export function formatDistance(distance: number | string | undefined | null): string {
    if (distance == null || distance === undefined) return `? km`;

    const num = typeof distance === "string" ? Number(distance) : distance;
    if (num == null || isNaN(num) || num < 0) return "--";
    if (num < 1000) {
        return `${Math.round(num)}m`;
    } else {
        const km = num / 1000;
        if (km > 999) return "999+km";
        return `${km.toFixed(1)}km`;
    }
}

export const formatCount = (count: number, limit: number = 99): string => {
    const num = Number(count);
    if (isNaN(num) || num <= 0) return "0";
    return num > limit ? `${limit}+` : num.toString();
};

type AssetNumberFormatOptions = {
    locale?: string;
    money?: boolean;
};

const trimTrailingZero = (value: string): string => {
    if (!value.includes(".")) return value;
    return value.replace(/0+$/, "").replace(/\.$/, "");
};

const EN_COMPACT_THRESHOLDS: [number, string][] = [
    [1_000_000_000, "B"],
    [1_000_000, "M"],
    [1_000, "K"]
];

// 中文量级后缀是 zh 分支的取值本身，en 走 EN_COMPACT_THRESHOLDS，不参与 i18n
const ZH_COMPACT_THRESHOLDS: [number, string][] = [
    [100_000_000, "亿"], // i18n-ignore
    [10_000, "万"] // i18n-ignore
];

function compactNumber(num: number, thresholds: [number, string][]): { scaled: number; suffix: string } | null {
    for (const [threshold, suffix] of thresholds) {
        if (Math.abs(num) >= threshold) {
            return { scaled: num / threshold, suffix };
        }
    }
    return null;
}

const intlFormatterCache = new Map<string, Intl.NumberFormat>();

function getCachedFormatter(locale: string, options: Intl.NumberFormatOptions): Intl.NumberFormat {
    const key = `${locale}|${JSON.stringify(options)}`;
    let fmt = intlFormatterCache.get(key);
    if (!fmt) {
        fmt = new Intl.NumberFormat(locale, options);
        intlFormatterCache.set(key, fmt);
    }
    return fmt;
}

/**
 * 资产数字国际化缩写格式化
 * - zh 用"万/亿"，en 用 K/M/B（手动实现，避免 SSR 与客户端水合时格式不一致）
 */
export function formatAssetNumberI18n(value: number | string, options: AssetNumberFormatOptions = {}): string {
    const { locale = "zh-CN", money = false } = options;
    const num = Number(value);

    if (!Number.isFinite(num)) {
        return money ? "0.00" : "0";
    }

    const isZh = locale.toLowerCase().startsWith("zh");
    const thresholds = isZh ? ZH_COMPACT_THRESHOLDS : EN_COMPACT_THRESHOLDS;
    const result = compactNumber(num, thresholds);

    if (!result) {
        return money ? num.toFixed(2) : String(Math.trunc(num));
    }

    return `${trimTrailingZero(result.scaled.toFixed(2))}${result.suffix}`;
}

/**
 * 资产数字国际化完整值格式化（用于 title 悬浮提示展示未缩写的精确值）
 */
export function formatAssetFullI18n(value: number | string, options: AssetNumberFormatOptions = {}): string {
    const { locale = "zh-CN", money = false } = options;
    const num = Number(value);
    if (!Number.isFinite(num)) {
        return money ? "0.00" : "0";
    }

    return getCachedFormatter(locale, {
        useGrouping: true,
        minimumFractionDigits: money ? 2 : 0,
        maximumFractionDigits: money ? 2 : 0
    }).format(num);
}
