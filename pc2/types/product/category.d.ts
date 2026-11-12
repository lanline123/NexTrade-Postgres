export interface CategoryNavigation {
    categoryId: number;
    categoryName: string;
    seoLink?: string;
    catList: {
        categoryId: number;
        categoryName: string;
        seoLink?: string;
    }[];
}
