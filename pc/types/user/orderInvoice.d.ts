// 列表查询时筛选参数类型
// 获取详情返回参数类型
export interface OrderInvoiceFormState {
    id?: number;
    invoiceId?: number;
    titleType?: number;
    confirm?: boolean;
    isConfirm?: number;
    invoiceType?: number;
    orderId?: number;
    isAdd?: number;
    status?: number;
    orderSn?: string;
    invoiceContent?: string;
    statusName?: string;
    totalAmount?: string;
    amount?: string;
    applyReply?: string;
    email?: string;
    mobile?: string;
    companyName?: string;
    companyUserName?: string;
    companyEnterpriseName?: string;
    companyCode?: string;
    companyAddress?: string;
    companyPhone?: string;
    companyBank?: string;
    companyAccount?: string;
    invoiceImg?: string;
    invoiceAttachment?: { picUrl: string; picName: string; picId: number; picThumb: string }[];
}

export interface OrderInvoiceFormResult {
    item: OrderInvoiceFormState;
}

// 编辑表单
