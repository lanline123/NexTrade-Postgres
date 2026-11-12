import { apiGet, apiPost } from '@/lib/client-api';

/**
 * 商品检索（列表 / 搜索 / 店铺内 / 图搜结果）共用口径。
 * Vue 基准：pages/search/components/search.vue → CatGoodsList.vue → api/search/search.ts
 *
 * **真实接口实测（2026-11-13）**：
 * - POST search/search/getProduct 的分类字段是 **`cat`**，不是 `categoryId`。
 *   传 `categoryId` 后端会忽略并返回全站商品（踩过的坑：`/list?categoryId=975` 之所以"能用"
 *   是恰好拿到了全量，而不是筛出分类）。
 * - 可用字段：`page` `size` `keyword` `cat` `pId` `brand`(逗号分隔) `sort`(sale|price|time)
 *   `order`(asc|desc) `min` `max` `attrs[{attrName, attrValue[]}]` `searchToken` `shopId`
 * - POST search/search/getFilter → { filter: { category[], brand[], shopCategory, maxPrice }, filterSelected }
 * - GET  search/search/getAttributeSuggestions?keyword=&cat= → { attributes: [{ name, values:[{count,value}] }] }
 */
export interface CatalogCategory {
    categoryId?: number | string;
    id?: number | string;
    categoryName?: string;
    parentId?: number;
    sortOrder?: number;
}

export interface CatalogBrand {
    brandId?: number | string;
    id?: number | string;
    brandName?: string;
    brandLogo?: string;
    firstWord?: string;
}

export interface CatalogAttrOption {
    value: string;
    count?: number;
}

export interface CatalogAttr {
    name: string;
    values: CatalogAttrOption[];
}

export interface FilterOptions {
    categories: CatalogCategory[];
    brands: CatalogBrand[];
    attrs: CatalogAttr[];
    maxPrice: number;
    /** 后端回显的已选条件（关键字/分类/品牌名），用于文案 */
    selected?: { keyword?: string | null; category?: string | null; brand?: string | null };
}

export type SortField = '' | 'sale' | 'price' | 'time';
export type SortOrder = '' | 'asc' | 'desc';

export interface SelectedAttr {
    attrName: string;
    attrValue: string[];
}

export interface CatalogQuery {
    keyword?: string;
    cat?: string;
    brand?: string;
    min?: number;
    max?: number;
    sort?: SortField;
    order?: SortOrder;
    attrs?: SelectedAttr[];
    shopId?: number;
    /** 店铺内分类（Vue pages/shopCategory 用这个字段，不是 cat） */
    shopCategoryId?: number;
    searchToken?: string;
    page?: number;
    size?: number;
}

export const categoryIdOf = (row: CatalogCategory): string => String(row.categoryId ?? row.id ?? '');
export const brandIdOf = (row: CatalogBrand): string => String(row.brandId ?? row.id ?? '');

export async function fetchFilterOptions(query: CatalogQuery): Promise<FilterOptions> {
    // 注意：**不要**给 getFilter 传 shopId —— 实测会 500（该接口不认店铺维度。
    // 店铺内的筛选是另一套 ShopFilter，本函数只服务平台列表/搜索。
    const result = await apiPost<{
        filter?: Record<string, unknown>;
        filterSelected?: { keyword?: string; category?: string; brand?: string };
    }>('search/search/getFilter', {
        keyword: query.keyword || undefined,
        cat: query.cat || undefined,
        brand: query.brand || undefined,
        min: query.min || undefined,
        max: query.max || undefined,
        size: query.size,
    });

    const filter = (result?.filter ?? {}) as Record<string, unknown>;
    const categories = (Array.isArray(filter.category) ? filter.category : []) as CatalogCategory[];
    const shopCategories = (Array.isArray(filter.shopCategory) ? filter.shopCategory : []) as CatalogCategory[];
    const brands = (Array.isArray(filter.brand) ? filter.brand : []) as CatalogBrand[];

    return {
        categories: categories.length ? categories : shopCategories,
        brands,
        attrs: [],
        maxPrice: Number(filter.maxPrice ?? 0) || 0,
        selected: result?.filterSelected ?? {},
    };
}

export async function fetchAttrSuggestions(query: CatalogQuery): Promise<CatalogAttr[]> {
    const result = await apiGet<{ attributes?: CatalogAttr[] }>('search/search/getAttributeSuggestions', {
        keyword: query.keyword || undefined,
        cat: query.cat || undefined,
        brand: query.brand || undefined,
    });
    return result?.attributes ?? [];
}
