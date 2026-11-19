/**
 * 询价口径（全站共享，与 PC 端 pc-semi/src/lib/inquiry.ts 逐字同口径）。
 *
 * 数据契约（后端下发，前端不自行推导 SKU 明细）：
 * - 商品带 `isInquiry`（0/1，商品级总开关）、`inquiryState`
 *   （"none" | "partial" | "all"：无 / 部分型号询价 / 全部型号询价）、
 *   `minPricedPrice`（有价型号的最低价，partial 时卡片展示用）。
 * - 详情 skuList 每行带 `isInquiry`；SKU 有效询价 = 行 isInquiry=1 或商品 isInquiry=1。
 * - 价格门控（第 2 期）：无看价权时后端把价格置空并在载荷打
 *   `priceVisible=false` + `priceGate=login|certification`。
 */

export type InquiryState = "none" | "partial" | "all";

/** 商品的询价派生态：优先用后端下发的 inquiryState，缺失时按商品级开关回落。 */
export function resolveInquiryState(product?: Record<string, any> | null): InquiryState {
    if (!product) return "none";
    const state = String(product.inquiryState ?? "");
    if (state === "all" || state === "partial" || state === "none") return state;
    return Number(product.isInquiry) === 1 ? "all" : "none";
}

/** SKU 有效询价（固定契约）：SKU 行 isInquiry=1 或商品 isInquiry=1，任一成立即询价。 */
export function skuIsInquiry(sku?: Record<string, any> | null, productIsInquiry?: unknown): boolean {
    return Number(sku?.isInquiry) === 1 || Number(productIsInquiry) === 1;
}

/** 给详情链接追加 ?inquiry=1（详情页挂载后自动打开留资表单）。 */
export function withInquiryParam(href: string): string {
    if (!href) return href;
    return href.includes("?") ? `${href}&inquiry=1` : `${href}?inquiry=1`;
}

/**
 * 价格门控文案：按载荷 priceGate 给出缺关文案与跳转目标。
 * 文案对齐原版 tigshop 共享价格组件口径（2026-11-20）：未登录
 * 「登录查看」、已登录未认证「实名查看」——与 format-price 组件
 * 内建的 B2B 文案逐字一致；第 2 期的自造文案（登录后查看价格/
 * 认证后查看价格）自此收敛。跳转目标保留我方既有形态（原版文案
 * 本身无跳转），不改变门控数据口径。
 */
export function priceGateCopy(priceGate?: string | null): { text: string; url: string } {
    return priceGate === "login"
        ? { text: "登录查看", url: "/pages/login/index" }
        : { text: "实名查看", url: "/pages/user/userCertification/index" };
}
