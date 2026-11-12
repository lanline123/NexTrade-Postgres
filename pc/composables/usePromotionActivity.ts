import { getCurrentScope, onScopeDispose, ref } from "vue";
import { getPromotion } from "~/api/product/product";
import type { PromotionList } from "~/types/product/product.d";

export type PromotionActivityInfo = Record<string, unknown>;

type PromotionSourceItem = {
    id?: string | number;
    productId?: string | number;
    shopId?: string | number;
};

type FetchPromotionOptions = {
    from?: string;
    shopId?: string | number;
    merge?: boolean;
    // 跨店铺商品集合需显式声明，避免 fallback 误用首条商品的 shopId 污染请求，
    // 否则后端会按单店铺过滤掉其它店铺的拼团等活动
    ignoreItemShopId?: boolean;
};

export function usePromotionActivity() {
    const promotionList = ref<Record<string, PromotionList>>({});
    let requestSerial = 0;

    if (getCurrentScope())
        onScopeDispose(() => {
            requestSerial++;
        });

    const getProductId = (item: PromotionSourceItem): string => {
        return String(item?.productId ?? item?.id ?? "");
    };

    const getActivityInfo = (productId: string | number): PromotionActivityInfo[] => {
        if (productId === undefined || productId === null || String(productId).trim() === "") return [];
        const activityInfo = promotionList.value[String(productId)]?.activityInfo;
        return Array.isArray(activityInfo) ? (activityInfo as PromotionActivityInfo[]) : [];
    };

    const clearPromotionList = (): void => {
        requestSerial++;
        promotionList.value = {};
    };

    const fetchPromotionList = async (items: PromotionSourceItem[], options: FetchPromotionOptions = {}): Promise<void> => {
        // 替换查询作废旧请求；追加查询属于同一批次，允许并行合并分页结果。
        const serial = options.merge ? requestSerial : ++requestSerial;
        const products = items
            .map((item) => getProductId(item))
            .filter((productId) => productId.trim() !== "")
            .map((productId) => ({ productId }));

        if (products.length === 0) {
            clearPromotionList();
            return;
        }

        const rawShopId = options.shopId ?? (options.ignoreItemShopId ? undefined : items.find((item) => item.shopId)?.shopId);

        try {
            const result = await getPromotion({
                products,
                shopId: rawShopId ? String(rawShopId) : undefined,
                from: options.from ?? "list"
            });

            // 清空、重新查询或离开页面后，旧请求的成功与失败都不再影响当前促销。
            if (serial !== requestSerial) return;
            promotionList.value = options.merge ? { ...promotionList.value, ...result } : (result as Record<string, PromotionList>);
        } catch {
            if (serial !== requestSerial) return;
            clearPromotionList();
        }
    };

    return {
        promotionList,
        getProductId,
        getActivityInfo,
        fetchPromotionList,
        clearPromotionList
    };
}
