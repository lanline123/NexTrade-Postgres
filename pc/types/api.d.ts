/** 网络响应外壳；request/asyncRequest 的泛型为解包后的 data。 */
export interface ApiEnvelope<T> {
    code: number;
    message: string;
    msg?: string;
    data: T;
}

export type MoneyAmount = string | number;
export type Timestamp = string | number;

// 首页
export interface HomeDataType {
    decorateId?: number;
    moduleList?: ModuleListType[];
}
export interface ModuleListType {
    module?: {};
    moduleIndex?: number;
    isShow?: boolean;
    active?: boolean;
    type?: string;
    style?: number;
}

export interface BaseResponseListWrap<T> {
    current: number;
    pages: number;
    records: T;
    size: number;
    total: number;
}
