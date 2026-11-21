export interface TabbarDataResponse {
    code: number;
    data: Item;
    message: string;
}

export interface Item {
    id: number;
    decorateSn: string;
    decorateName: string;
    data: Data;
}

export interface Data {
    navList: List[];
    picList: Array<any[] | List>;
    backgroundColor: string;
    backgroundPic: BackgroundPic;
}

export interface BackgroundPic {
    picUrl: string;
    picThumb: string;
    picId: number;
    picName: string;
}

export interface List {
    picActiveThumb: string;
    picActiveUrl: string;
    picId: number;
    picThumb: string;
    picUrl: string;
    picName: string;
    picTitle: string;
    picLink?: PicLink;
    /** 入口类型（我方新增）：'qrcode' = 扫码入口，缺省 = 链接跳转 */
    entryType?: string;
    /** 二维码类型（我方新增）：'wecom' 企业微信 / 'official' 公众号 */
    qrType?: string;
    /** 二维码图片 URL（我方新增） */
    qrImage?: string;
}

export interface PicLink {
    path: string;
    label: string;
    name: string;
    link: string;
    appLink: string;
}
