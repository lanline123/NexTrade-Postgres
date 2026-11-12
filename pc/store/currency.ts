import { getCurrencyList } from "@/api/common";
import type { CurrencyItem } from "@/types/common/common.d.ts";

export const useCurrencyStore = defineStore("currency", () => {
    const currencyList = ref<CurrencyItem[]>([]);
    // const currentCurrencyData = ref<CurrencyItem>({} as CurrencyItem);
    const currentCurrencyData = useCookie<CurrencyItem | null>("currentCurrencyData");
    const fetchCurrencyList = async () => {
        try {
            const result = await getCurrencyList();
            currencyList.value = result;
            if (!currentCurrencyData.value) {
                currentCurrencyData.value = result?.find((item) => item.isDefault === 1)!;
            }
        } catch (error: any) {
            console.error(error.message);
        }
    };

    // 返回响应式属性和方法
    return {
        currencyList,
        fetchCurrencyList
    };
});
