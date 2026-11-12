/**
 * 搜索页筛选数据 hook
 * 统一封装「筛选项请求 + 归一化 + 状态」，供左侧筛选（FilterSide）使用。
 * 顶部 Filter.vue 保持自有逻辑不变，本 hook 仅服务新组件。
 */
import { reactive, ref } from "vue";

import { searchApi } from "./api";
import { resolveBrandList, resolveCategoryList, resolveMaxPrice, type SearchFilterAttrItem } from "./searchFilterUtils";
import type { filterSeleted, SearchFilterBrandRow, ApiSearchCategoryVO } from "./types.d";

interface SearchFilterListState {
    brand: SearchFilterBrandRow[];
    category: ApiSearchCategoryVO[];
    attrs: SearchFilterAttrItem[];
}

export function useSearchFilterData() {
    const filterList = reactive<SearchFilterListState>({
        brand: [],
        category: [],
        attrs: []
    });
    const filterSeleted = ref<filterSeleted>();
    const maxPrice = ref(0);
    const sliderMax = ref(0);
    const loading = ref(false);

    /** 拉取品牌 / 分类 / 价格上限（home/home/pcSearch 同源接口 getSearchFilter） */
    const fetchFilter = async (params: Record<string, unknown>): Promise<void> => {
        if (loading.value) return;
        try {
            loading.value = true;
            const result = await searchApi.getFilter(params);
            filterList.brand = resolveBrandList(result?.filter) ?? [];
            filterList.category = resolveCategoryList(result?.filter) ?? [];
            const mp = resolveMaxPrice(result?.filter);
            maxPrice.value = mp;
            if (sliderMax.value < mp) {
                sliderMax.value = mp;
            }
            filterSeleted.value = (result?.filterSelected ?? {}) as filterSeleted;
        } catch (error) {
            console.error("[useSearchFilterData.fetchFilter] 获取搜索筛选项失败:", error);
        } finally {
            loading.value = false;
        }
    };

    /** 拉取属性筛选项（ES），并初始化勾选态 */
    const fetchEsFilter = async (params: Record<string, unknown>): Promise<void> => {
        try {
            const result = (await searchApi.getEsFilter(params)) as { attributes?: SearchFilterAttrItem[] };
            (result?.attributes ?? []).forEach((item) => {
                (item.values ?? []).forEach((attr) => {
                    attr.checked = false;
                });
            });
            filterList.attrs = result?.attributes ?? [];
        } catch (error) {
            console.error("[useSearchFilterData.fetchEsFilter] 获取属性筛选项失败:", error);
        }
    };

    return {
        filterList,
        filterSeleted,
        maxPrice,
        sliderMax,
        loading,
        fetchFilter,
        fetchEsFilter
    };
}
