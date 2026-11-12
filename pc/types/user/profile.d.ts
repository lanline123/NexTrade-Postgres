export interface ProfileFormResult {
    item: ProfileFormState;
}

// 获取详情返回参数类型
export interface ProfileFormState {
    userId?: number;
    birthday?: string;
    nickname?: string;
    email?: string;
    mobile?: string;
}
