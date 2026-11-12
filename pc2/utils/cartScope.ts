export const mapCartLineShopIds = <T extends { shopId?: number; carts: Array<Record<string, any>> }>(groups: T[]): T[] =>
    groups.map((group) => ({
        ...group,
        carts: group.carts.map((line) => ({ ...line, shopId: line.shopId ?? group.shopId ?? 0 }))
    }));

export const buildCartProductRoute = (line: { productId?: number; productSn?: string; shopId?: number }) => ({
    path: "product",
    id: line.productId,
    sn: line.productSn,
    shopId: Number(line.shopId ?? 0)
});
