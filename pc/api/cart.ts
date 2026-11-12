import { applyCartSaleState } from "@/utils/productSale";
import { request, asyncRequest } from "@/utils/request";
import type { CartFilterResult } from "@/types/cart/cart.d";

/**
 * @description 获取购物车列表
 * @returns 购物车数据
 */
export const getCart = () => {
    return asyncRequest<CartFilterResult>({
        url: "cart/cart/list",
        method: "get",
        noSkipLogin: true
    }).then(applyCartSaleState);
};

/**
 * @description 更新购物车商品选中状态
 * @param data 包含商品ID、选中状态
 * @returns 操作结果
 */
export const updateCartCheck = (data: any) => {
    return asyncRequest({
        url: "cart/cart/updateCheck",
        method: "post",
        data: data
    });
};

/**
 * @description 更新购物车商品数量
 * @param cartId 购物车项ID
 * @param data 包含数量等
 * @returns 更新后的购物车数据
 */
export const updateCartItemData = (cartId: any, data: { quantity?: number }) => {
    return asyncRequest({
        url: "cart/cart/updateItem",
        method: "post",
        data: { cartId, data }
    });
};

/**
 * @description 移除购物车商品
 * @param cartIds 要移除的购物车项ID数组
 * @returns 操作结果
 */
export const removeCartItemData = (cartIds: number[]) => {
    return asyncRequest({
        url: "cart/cart/removeItem",
        method: "post",
        data: { cartIds }
    });
};

/**
 * @description 清空购物车
 * @returns 操作结果
 */
export const clearCart = () => {
    return asyncRequest({
        url: "cart/cart/clear",
        method: "post"
    });
};

/**
 * @description 获取购物车商品数量
 * @returns 购物车商品总数
 */
export const asyncGetCartCount = () => {
    return asyncRequest<any>({
        url: "cart/cart/getCount",
        method: "get",
        noSkipLogin: true
    });
};
