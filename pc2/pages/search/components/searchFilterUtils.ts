/**
 * 搜索页筛选 - 纯函数工具
 * 从后端筛选数据中归一化出品牌 / 分类 / 已选属性，供左侧筛选（FilterSide）等新组件复用。
 * 说明：现有顶部 Filter.vue 内部已有等价逻辑，这里独立一份以服务新代码，不回改 Filter.vue。
 */
import type { SearchFilterBrandRow, ApiSearchCategoryVO } from "./types.d";

/** 带勾选态的属性筛选项（ES 返回结构） */
export interface SearchFilterAttrOption {
    value: string;
    checked?: boolean;
}

export interface SearchFilterAttrItem {
    name: string;
    values: SearchFilterAttrOption[];
}

/** 已选属性（提交给后端的结构） */
export interface SelectedAttr {
    attrName: string;
    attrValue: string[];
}

/** 从筛选数据里归一化品牌列表 */
export const resolveBrandList = (filterData: unknown): SearchFilterBrandRow[] => {
    const data = (filterData || {}) as Record<string, unknown>;
    const rawList = (Array.isArray(data.brand) && data.brand) || (Array.isArray(data.brands) && data.brands) || [];

    return rawList
        .map((item) => {
            const row = (item || {}) as Record<string, unknown>;
            return {
                id: String(row.id ?? row.brandId ?? ""),
                brandName: String(row.brandName ?? row.name ?? ""),
                brandLogo: String(row.brandLogo ?? row.logo ?? ""),
                firstWord: String(row.firstWord ?? row.initial ?? row.first_word ?? row.brandFirstWord ?? row.letter ?? "")
            };
        })
        .filter((item) => item.id && item.brandName);
};

/** 从筛选数据里归一化分类列表（左侧布局不展示分类，但保留以备复用） */
export const resolveCategoryList = (filterData: unknown): ApiSearchCategoryVO[] => {
    const data = (filterData || {}) as Record<string, unknown>;
    const rawList =
        (Array.isArray(data.category) && data.category) ||
        (Array.isArray(data.categories) && data.categories) ||
        (Array.isArray(data.shopCategory) && data.shopCategory) ||
        [];

    return rawList
        .map((item) => {
            const row = (item || {}) as Record<string, unknown>;
            const id = String(row.id ?? row.categoryId ?? "");
            const categoryName = String(row.categoryName ?? row.name ?? "");
            const categoryPic = String(row.categoryPic ?? row.pic ?? row.image ?? row.icon ?? "");
            const seoLink = typeof row.seoLink === "string" ? row.seoLink : undefined;
            return { id, categoryName, categoryPic, seoLink };
        })
        .filter((item) => item.id && item.categoryName);
};

/** 从带勾选态的属性列表中提炼已选属性（提交结构） */
export const buildSelectedAttrs = (attrs: SearchFilterAttrItem[]): SelectedAttr[] => {
    return (attrs || [])
        .map((attr) => {
            const selectedValues = (attr.values || []).filter((item) => item.checked).map((item) => item.value);
            if (selectedValues.length > 0) {
                return { attrName: attr.name, attrValue: [...selectedValues] };
            }
            return null;
        })
        .filter((item): item is SelectedAttr => Boolean(item));
};

/** 提取筛选数据里的最高价（用于价格滑块上限） */
export const resolveMaxPrice = (filterData: unknown): number => {
    const data = (filterData || {}) as Record<string, unknown>;
    return Number((data.maxPrice as number | undefined) ?? 0) || 0;
};
