/**
 * 版本判断（对齐 view/admin utils/version 的原版口径）
 *
 * 原版 utils/version 的 isPro/isStore/isB2b/isOverseas/isMerchant 均为 VITE_*
 * 构建期「版本（edition）标志」，与登录账号无关；admin-semi 同口径以构建期
 * 变量为准（开关统一在 admin-semi/.env，本产品全量版全部打开）。
 * 账号/上下文判断是另一回事：isAdminType（adminType）、isShopEnd（shopId 上下文）、
 * isS2b2c（供应商账号）。
 *
 * 历史教训（2026-11-21 点单卡片复核）：isStore 曾被误写为 adminType==="shop"
 * 的账号判断，导致平台管理员在装修模块库里看不到首焦轮播A2/门店导航/点单卡片
 * （原版 enabled: isPro() && isStore() 在多门店版恒真），已按原版口径改正。
 */

export function isAdminType(): boolean {
    const t = localStorage.getItem("adminType") || "admin";
    return t === "admin";
}

/** 是否多门店版（原版 VITE_IS_STORE 版本标志；本产品全量版 .env 置 1） */
export function isStore(): boolean {
    return String(import.meta.env.VITE_IS_STORE ?? "") === "1";
}

/** 是否供应商账号（账号判断，非原版 VITE_IS_VENDOR 版本标志；编辑器内无 enabled 消费者） */
export function isS2b2c(): boolean {
    return localStorage.getItem("adminType") === "vendor";
}

/** 是否多商户形态（承接原版 VITE_IS_MERCHANT 版本标志：全量版下恒真，商品详情店铺信息块对所有账号可见） */
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
