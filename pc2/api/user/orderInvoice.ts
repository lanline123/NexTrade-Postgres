import type { OrderInvoiceFormResult } from "~/types/user/orderInvoice";

export const getOrderInvoice = (params: object) => {
    return asyncRequest<OrderInvoiceFormResult>({
        url: "user/orderInvoice/detail",
        method: "get",
        params
    });
};

export const insertOrderInvoice = (data: object, type: string) => {
    return asyncRequest<OrderInvoiceFormResult>({
        url: "user/orderInvoice/" + type,
        method: "post",
        data
    });
};

export const updateOrderInvoice = (data: object) => {
    return asyncRequest<OrderInvoiceFormResult>({
        url: "user/orderInvoice/update",
        method: "post",
        data
    });
};

export const delOrderInvoice = (data: object) => {
    return asyncRequest<OrderInvoiceFormResult>({
        url: "user/orderInvoice/del",
        method: "post",
        data
    });
};
