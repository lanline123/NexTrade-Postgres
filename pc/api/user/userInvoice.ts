import type { UserInvoiceStatus, UserInvoiceFormState } from "~/types/user/userInvoice";

//获取增票资质详情
export const getUserInvoice = () => {
    return asyncRequest<UserInvoiceFormState>({
        url: "user/invoice/detail",
        method: "get"
    });
};
export const updateUserInvoice = (data: object) => {
    return asyncRequest({
        url: "user/invoice/update",
        method: "post",
        data
    });
};
export const invoiceUserInvoice = (data: object) => {
    return asyncRequest({
        url: "user/invoice/create",
        method: "post",
        data
    });
};
export const getInvoiceStatus = () => {
    return asyncRequest<UserInvoiceStatus>({
        url: "user/invoice/getStatus",
        method: "get"
    });
};
