// 获取列表返回参数类型
export interface FriendLinksFilterState {
    linkId: number;
    linkLogo: string;
    linkTitle: string;
    linkUrl: string;
    sortOrder: number;
}

export interface FriendLinksFilterResult {
    list: FriendLinksFilterState[];
}
