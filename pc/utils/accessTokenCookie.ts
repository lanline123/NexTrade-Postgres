/** 登录态 Cookie 名 */
export const ACCESS_TOKEN_COOKIE = "accessToken";

/** 登录态 Cookie 有效期：30 天 */
export const ACCESS_TOKEN_MAX_AGE = 60 * 60 * 24 * 30;

const TOKEN_COOKIE_PATH = "/";
const TOKEN_COOKIE_SAME_SITE = "Lax";
const EXPIRED_COOKIE_DATE = "Thu, 01 Jan 1970 00:00:00 GMT";

export type AccessTokenCookieOp =
    | { type: "expire"; domain?: string }
    | { type: "set"; value: string; domain?: string };

/**
 * 生成登录态 Cookie 的写入序列：先删可能存在的同名旧票，再写入唯一一份新票。
 * 同名 Cookie 若 Domain 不同会被浏览器当成两份，刷新时解析器只认第一份。
 */
export const buildAccessTokenCookieOps = (value: string | null | undefined, sharedDomain?: string): AccessTokenCookieOp[] => {
    const ops: AccessTokenCookieOp[] = [{ type: "expire" }];
    if (sharedDomain) ops.push({ type: "expire", domain: sharedDomain });
    if (value) ops.push(sharedDomain ? { type: "set", value, domain: sharedDomain } : { type: "set", value });
    return ops;
};

const appendCookieAttrs = (parts: string[], domain?: string) => {
    parts.push(`Path=${TOKEN_COOKIE_PATH}`);
    parts.push(`SameSite=${TOKEN_COOKIE_SAME_SITE}`);
    if (domain) parts.push(`Domain=${domain}`);
};

export const serializeAccessTokenCookieOp = (op: AccessTokenCookieOp): string => {
    if (op.type === "expire") {
        const parts = [`${ACCESS_TOKEN_COOKIE}=`, "Max-Age=0", `Expires=${EXPIRED_COOKIE_DATE}`];
        appendCookieAttrs(parts, op.domain);
        return parts.join("; ");
    }
    const parts = [`${ACCESS_TOKEN_COOKIE}=${encodeURIComponent(op.value)}`, `Max-Age=${ACCESS_TOKEN_MAX_AGE}`];
    appendCookieAttrs(parts, op.domain);
    return parts.join("; ");
};
