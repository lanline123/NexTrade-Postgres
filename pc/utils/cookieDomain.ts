/** 各语言独立域名的共享状态键，由语言列表接口填充 */
export const LOCALE_DOMAINS_STATE_KEY = "i18n:localeDomains";

/**
 * 求可共享的 Cookie Domain
 * @description 取当前访问域名与各语言域名的最长公共后缀。
 *  公共后缀不足两段说明分属不同主域（或本地、IP 直连），此时无法共享，返回 undefined。
 * @param host 当前访问域名，不含端口
 * @param domains 各语言配置的独立域名
 * @returns Domain 值，形如 `.example.com`；不可共享时为 undefined
 */
export const resolveSharedCookieDomain = (host: string, domains: string[]): string | undefined => {
    const candidates = [...new Set([host, ...domains].map((item) => String(item || "").trim().toLowerCase()).filter(Boolean))];
    if (candidates.length < 2) return undefined;

    const reversed = candidates.map((item) => item.split(".").reverse());
    const minLength = Math.min(...reversed.map((segments) => segments.length));

    const common: string[] = [];
    for (let index = 0; index < minLength; index++) {
        const segment = reversed[0][index];
        if (!reversed.every((segments) => segments[index] === segment)) break;
        common.push(segment);
    }

    if (common.length < 2) return undefined;
    return `.${common.reverse().join(".")}`;
};

/**
 * 当前站点可共享的 Cookie Domain
 * @description 语言域名列表由接口异步返回，尚未就绪时返回 undefined，Cookie 保持 host-only。
 *  因此需要共享的 Cookie 必须在写入时刻调用本函数，不能在模块初始化时取值。
 */
export const sharedCookieDomain = (): string | undefined => {
    const domains = useState<string[]>(LOCALE_DOMAINS_STATE_KEY, () => []).value;
    if (!domains.length) return undefined;
    return resolveSharedCookieDomain(useRequestURL().hostname, domains);
};
