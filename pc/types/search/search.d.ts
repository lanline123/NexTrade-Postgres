import type { ProductItem } from "@/types/product/product.d";
export interface filterSeleted {
    brand?: string;
    category?: string;
    attrs: any[];
}
export interface filterSeletedData {
    brand?: string;
    category?: string;
    attrs: any[];
}
export interface SearchFilter {
    page: number;
    categoryId: number;
    brandIds: number[];
    sortField: string;
    sortOrder: string;
    couponId: number;
    minPrice: number | null;
    maxPrice: number | null;
    keyword: string;
}
export interface SearchFilterResult {
    filter: SearchFilterListResult;
    filterSelected: filterSeleted;
}

export interface SearchFilterListResult {
    brand: {
        brandName: string;
        brandId: number;
        brandLogo: string;
        firstWord: string;
    }[];
    category: {
        categoryName: string;
        categoryId: number;
        seoLink?: string;
    }[];
}

export interface SearchProductListResult {
    records: ProductItem[];
    total: number;
}

export interface QueryParams {
    cat: number;
    brand: string;
    sort: string;
    order: string;
    page: number;
    max: number;
    min: number;
    attrs?: string;
    keyword: string;
    [key: string]: string | number | string[] | undefined;
}
