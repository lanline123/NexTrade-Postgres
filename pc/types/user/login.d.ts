export interface quickLoginSettingResult {
    showOauth: number;
    wechatLogin: number;
}

export interface wechatLoginUrlResult {
    url: string;
    ticket: string;
}

export interface wechatEventResult {
    type: number;
    message: string;
    token: string;
    openData: any;
}

export interface bindMobileResult {
    token: string;
}
