// 列表查询时筛选参数类型
import type { SkuData } from "~/types/product/product";
import type { OrderFormState } from "~/types/user/order";
import type { BaseResponseListWrap } from "~/types/api";

export interface AfterSalesFilterParams {
    page: number;
    size?: number;
}

// 获取列表返回参数类型
export interface AfterSalesFilterState {
    orderId: number;
    orderSn: string;
    shippingTime: string;
    items: AfterSalesItemsState[];
}

export interface AfterSalesItemsState {
    itemId: number;
    aftersaleFlag: number;
    orderId: number;
    productId: number;
    aftersaleId: number;
    price: string;
    quantity: number;
    productName: string;
    productSn: string;
    picThumb: string;
    productPicThumb: string;
    aftersalesItem: object;
    productStock: number;
    productWeight: string;
    toAftersalses: boolean;
}

export interface AfterSalesFilterResult extends BaseResponseListWrap<AfterSalesFilterState[]> {
    filter: AfterSalesFilterParams;
    records: AfterSalesFilterState[];
}

// 获取详情返回参数类型
export interface AfterSalesFormState {
    itemId?: number;
    orderId?: number;
    orderStatus?: number;
    isPickup?: boolean;
    canApplyQuantity?: number;
    orderItemId?: number;
    aftersaleReason?: string;
    refundAmount: number;
    refundAmountMax: number;
    picThumb?: string;
    pics: PicsList[];
    picsList?: PicsList[];
    productSn?: string;
    productName?: string;
    price?: string;
    quantity?: number;
    subtotal?: string;
    aftersaleType?: number;
    aftersaleTypeList?: object;
    number?: number;
    description?: string;
    skuData?: SkuData[];
    items: Items[];
    isGift?: number;
}
interface Items {
    orderItemId?: number;
    number?: number;
}
export interface PicsList {
    picThumb: string;
    picName: string;
    picUrl: string;
}
export interface AfterSalesFormResult {
    list: AfterSalesFormState[];
    aftersaleType: object;
    aftersaleReason: object;
    order: OrderFormState;
}

// 编辑表单
