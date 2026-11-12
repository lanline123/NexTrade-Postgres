// 列表查询时筛选参数类型
// 获取详情返回参数类型
export interface SecurityFormState {
    mobile?: string;
    oldMobile?: string;
    mobileCode?: string;
    newMobile?: string;
    newMobileCode?: string;
    oldPassword?: string;
    password?: string;
    confirmPassword?: string;
    email?: string;
    emailCode?: string;
    newEmail?: string;
    newEmailCode?: string;
}

export interface SecurityFormResult {
    item: SecurityFormState;
}
