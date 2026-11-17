/**
 * 版本判断（对齐 view/admin utils/version 的运行时口径）
 *
 * view/admin 用 VITE_* 构建期常量；admin-semi 统一用登录态 adminType 判断：
 * - admin = 平台总后台
 * - shop  = 多门店店铺端（isStore）
 * - vendor = 供应商端（isS2b2c）
 */

export function isAdminType(): boolean {
    const t = localStorage.getItem("adminType") || "admin";
    return t === "admin";
}

/** 是否店铺端（多门店） */
export function isStore(): boolean {
    return localStorage.getItem("adminType") === "shop";
}

/** 是否供应商端 */
export function isS2b2c(): boolean {
    return localStorage.getItem("adminType") === "vendor";
}

/** 是否商家端（店铺/供应商统称） */
export function isMerchant(): boolean {
    return isStore() || isS2b2c();
}

/**
 * 是否店铺端上下文（原版各模块 index.ts 内联的 isShopEnd 同口径）：
 * localStorage shopId 非空且非 "0" 时为 true；catNav/searchBar/searchBar2 等
 * 模块以 enabled: () => !isShopEnd() 在店铺端隐藏。
 */
export function isShopEnd(): boolean {
    const shopId = localStorage.getItem("shopId");
    return !!shopId && shopId !== "0";
}

/** 是否 B2B 版（与 view/admin 的 VITE_IS_B2B 对齐；admin-semi 以构建期变量为准，缺省关闭） */
export function isB2b(): boolean {
    return String(import.meta.env.VITE_IS_B2B ?? "") === "1";
}

/** 是否跨境版 */
export function isOverseas(): boolean {
    return String(import.meta.env.VITE_IS_OVERSEAS ?? "") === "1";
}
