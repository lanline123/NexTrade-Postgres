/**
 * 商品卡片加购判定（自旧 hooks/useProductCardBuy.ts 逐函数等价移植，零依赖纯函数）。
 * 旧文件导出 useProductCardBuy() => { isWholesaleCard, cardProductId }，
 * 供商品卡片区分批发卡与取商品 ID；React 版保留同名 hook 包装，
 * 同时直接导出两个纯函数便于测试与非组件代码复用。
 * 实际加购执行器在 components/product/buy（规格弹窗/跳结算），不在本 hook 内。
 */

export interface CardProduct {
    productId?: number | string;
    id?: number | string;
    firstWholesalePrice?: any;
    [key: string]: any;
}

/** 批发卡判定：存在 firstWholesalePrice 字段即为批发商品卡（旧逻辑原文）。 */
export function isWholesaleCard(item?: CardProduct | null): boolean {
    return !!item && item.firstWholesalePrice !== undefined && item.firstWholesalePrice !== null;
}

/** 卡片商品 ID：优先 productId，回落 id，缺省 0（旧逻辑原文）。 */
export function cardProductId(item?: CardProduct | null): number {
    return Number(item?.productId ?? item?.id ?? 0);
}

export const useProductCardBuy = () => {
    return { isWholesaleCard, cardProductId };
};

export default useProductCardBuy;
