import { ref, watch, computed, type Ref, type ComputedRef } from "vue";
import { batchActiveStyle } from "~/api/product/productStyle";
import type { ProductActiveStyleVO } from "~/types/product/productStyle";

const globalStyleCache = new Map<number, ProductActiveStyleVO>();
const pendingRequests = new Map<string, Promise<Record<number, ProductActiveStyleVO> | null>>();

/**
 * 商品样式（边框/活动背景）composable
 * 监听商品列表变化，自动批量获取商品生效样式并合并到 styleMap
 * 内部通过模块级缓存去重，同一页面多个组件挂载时不会重复请求相同商品 ID
 */
export function useProductStyle(productList: Ref<any[]> | ComputedRef<any[]>) {
    const styleMap = ref<Record<number, ProductActiveStyleVO>>({});
    let lastFetchedLength = 0;

    const fetchStyles = async (products: any[]) => {
        if (!products || products.length === 0) return;
        const allIds = products
            .map((p: any) => p.productId || p.id)
            .filter(Boolean) as number[];
        if (allIds.length === 0) return;

        const cached: Record<number, ProductActiveStyleVO> = {};
        const uncachedIds: number[] = [];
        for (const id of allIds) {
            if (globalStyleCache.has(id)) {
                cached[id] = globalStyleCache.get(id)!;
            } else {
                uncachedIds.push(id);
            }
        }

        if (Object.keys(cached).length > 0) {
            styleMap.value = { ...styleMap.value, ...cached };
        }

        if (uncachedIds.length === 0) return;

        const requestKey = uncachedIds.sort((a, b) => a - b).join(",");

        try {
            let promise = pendingRequests.get(requestKey);
            if (!promise) {
                promise = batchActiveStyle(requestKey).then((result) => {
                    pendingRequests.delete(requestKey);
                    if (result && typeof result === "object") {
                        for (const [idStr, style] of Object.entries(result)) {
                            globalStyleCache.set(Number(idStr), style);
                        }
                        return result;
                    }
                    return null;
                }).catch(() => {
                    pendingRequests.delete(requestKey);
                    return null;
                });
                pendingRequests.set(requestKey, promise);
            }

            const result = await promise;
            if (result) {
                styleMap.value = { ...styleMap.value, ...result };
            }
        } catch {
            // 样式获取失败不影响主流程
        }
    };

    watch(
        () => productList.value,
        (newList) => {
            if (!newList || newList.length === 0) {
                styleMap.value = {};
                lastFetchedLength = 0;
                return;
            }
            if (newList.length > lastFetchedLength) {
                const newItems = newList.slice(lastFetchedLength);
                lastFetchedLength = newList.length;
                fetchStyles(newItems);
            }
        },
        { immediate: true }
    );

    const resetStyles = () => {
        styleMap.value = {};
        lastFetchedLength = 0;
    };

    return { styleMap, fetchStyles, resetStyles };
}

export default useProductStyle;
