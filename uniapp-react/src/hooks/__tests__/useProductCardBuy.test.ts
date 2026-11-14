import { describe, expect, it } from "vitest";

import { cardProductId, isWholesaleCard, useProductCardBuy } from "../useProductCardBuy";

/**
 * 本线新增 hook（自旧 hooks/useProductCardBuy.ts 等价移植）的契约测试。
 * 旧文件零依赖纯函数：批发卡按 firstWholesalePrice 字段存在判定，
 * 商品 ID 优先 productId 回落 id。
 */
describe("useProductCardBuy", () => {
    it("isWholesaleCard 按 firstWholesalePrice 存在性判定", () => {
        expect(isWholesaleCard({ firstWholesalePrice: 10 })).toBe(true);
        expect(isWholesaleCard({ firstWholesalePrice: 0 })).toBe(true);
        expect(isWholesaleCard({})).toBe(false);
        expect(isWholesaleCard(null)).toBe(false);
        expect(isWholesaleCard(undefined)).toBe(false);
    });

    it("cardProductId 优先 productId、回落 id、缺省 0", () => {
        expect(cardProductId({ productId: 5, id: 9 })).toBe(5);
        expect(cardProductId({ id: 9 })).toBe(9);
        expect(cardProductId({ productId: "12" })).toBe(12);
        expect(cardProductId(null)).toBe(0);
    });

    it("hook 包装返回同两个函数", () => {
        const api = useProductCardBuy();
        expect(api.isWholesaleCard).toBe(isWholesaleCard);
        expect(api.cardProductId).toBe(cardProductId);
    });
});
