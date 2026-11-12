import { asyncGetCartCount, getCart, removeCartItemData } from "@/api/cart";

export const useCartStore = defineStore("cart", () => {
    const cartList = ref<any[]>([]);
    const cartCount = ref(0);
    /** 进行中的角标请求，用于多实例去重，避免同一时刻重复请求 getCount */
    let cartCountInflight: Promise<number> | null = null;

    /** 统一解析 getCount 返回：兼容 number / { totalCount } / { total: { totalCount } } */
    function parseCartCount(result: any): number {
        if (typeof result === "number") return result;
        if (result && typeof result.totalCount === "number") return result.totalCount;
        if (result && result.total && typeof result.total.totalCount === "number") return result.total.totalCount;
        return 0;
    }

    /**
     * 拉取购物车角标数量
     * @description 统一入口，多实例并发时复用同一请求，结果写入 cartCount
     */
    async function fetchCartCount(): Promise<number> {
        if (cartCountInflight) return cartCountInflight;
        cartCountInflight = (async () => {
            try {
                const result = await asyncGetCartCount();
                cartCount.value = parseCartCount(result);
                return cartCount.value;
            } finally {
                cartCountInflight = null;
            }
        })();
        return cartCountInflight;
    }

    async function changeCartList() {
        try {
            const result = await getCart();
            cartList.value = result.cartList;
            cartCount.value = parseCartCount(result);
        } catch (error) {
        } finally {
        }
    }

    // 购物车数量统一入口：兼容后端返回裸数字与 {totalCount} 两种形态
    async function fetchCartCount() {
        try {
            const result: any = await asyncGetCartCount();
            if (typeof result === "number") cartCount.value = result;
            else if (result && typeof result.totalCount === "number") cartCount.value = result.totalCount;
        } catch (error) {}
    }

    function resetCart() {
        cartList.value = [];
        cartCount.value = 0;
    }

    return {
        cartList,
        cartCount,
        changeCartList,
        fetchCartCount,
        resetCart
    };
});
