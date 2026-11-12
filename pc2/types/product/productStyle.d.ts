export interface ProductActiveStyleVO {
    borderStyleId: number | null;
    borderImage: string | null;
    borderPcImage: string | null;
    activityStyleId: number | null;
    activityStyleName: string | null;
    activityBackgroundImage: string | null;
    activityBackgroundPcImage: string | null;
}

export interface ProductActiveStyleResponse {
    code: number;
    message: string;
    data: ProductActiveStyleVO;
}

export type BatchActiveStyleData = Record<number, ProductActiveStyleVO>;

export interface BatchActiveStyleResponse {
    code: number;
    message: string;
    data: BatchActiveStyleData;
}
