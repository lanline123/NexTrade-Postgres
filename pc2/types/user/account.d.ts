import type { BaseResponseListWrap } from "~/types/api";
// 列表查询时筛选参数类型
export interface AccountFilterParams {
    page: number;
    size: number;
    status: number;
    accountType: number;
}

// 获取列表返回参数类型
export interface AccountFilterState {
    addTime: string;
    amount: string;
    type: string;
    postscript: string;
    statusType: string;
    addTimeLong: number;
    status: number;
}

export interface AccountDetailFilterState {
    logId: number;
    userId: number;
    username: string;
    balance: number;
    beforeBalance: number;
    afterBalance: number;
    frozenBalance: number;
    beforeFrozenBalance: number;
    afterFrozenBalance: number;
    changeTime: string;
    changeDesc: string;
    changeType: number;
    changeTypeName: string;
}

export interface AccountFilterResult {
    records: AccountFilterState[];
    filter: AccountFilterParams;
    total: number;
}

export interface AccountDetailFilterResult extends BaseResponseListWrap<AccountDetailFilterState[]> {
    filter: AccountFilterParams;
}
// 充值

// 获取列表返回参数类型
export interface DepositFilterState {
    discountMoney: string;
    money: string;
    rechargeId: number;
    isShow: number;
    selected: boolean;
}

export interface DepositFilterParams {
    page?: number;
    size?: number;
    sortField?: string;
    sortOrder?: string;
}

export interface DepositFilterResult {
    records: DepositFilterState[];
    filter: DepositFilterParams;
    total: number;
}

// 获取详情返回参数类型
export interface AccountFormState {
    amount: string;
    accountData: AccountData;
}

export interface AccountData {
    accountType: number;
    accountName: string;
    accountNo: string;
    identity: string;
    bankName: string;
}

export interface AccountInfoResult {
    records: AccountInfo[];
    filter: object;
}

export interface AccountInfo {
    accountTypeName: string;
    accountId: number;
    userId: number;
    accountType: number;
    accountName: string;
    accountNo: string;
    identity: string;
    bankName: string;
}
export interface AccountFormResult {
    accountDetail: AccountInfo;
}
