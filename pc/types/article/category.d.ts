import type { ArticleFilterState } from "~/types/article/article";

export interface CategoryFilterParams {
    page: number;
    size: number;
    sortField?: string;
    sortOrder?: string;
    keyword?: string;
    categorySn?: string;
}

export interface CategoryFilterState {
    articleCategoryId: number;
    articleCategoryName: string;
    categorySn: string;
    categoryType: string;
    parentId: number;
    articles: Article[];
}

export interface Article {
    articleId: number;
    articleTitle: string;
    articleCategoryId: number;
    articleSn: string;
    articleThumb: string;
    articleTag: string;
    articleAuthor: string;
    articleType: number;
    content: string;
    description: string;
    keywords: string;
    isShow: number;
    addTime: number;
    isHot: number;
    isTop: number;
    clickCount: number;
    link: string;
}

export interface CategoryFormState {
    consignee?: string;
    regionIds?: number[];
    regionName?: string;
    addressId?: number;
    address?: string;
    email?: string;
    telephone?: string;
    mobile?: string;
}
export interface CategoryFilterResult {
    // records: CategoryFilterState[];
    // filter: CategoryFilterParams;
    // total: number;
}

export interface CategoryFormResult {
    item: CategoryFormState;
}
