import type { BaseResponseListWrap } from "~/types/api";
export interface UserMsgFilterParams {
    page: number;
    size?: number;
    unread: number;
}

// 获取列表返回参数类型
export interface UserMsgFilterState {
    addTimeDate: string;
    addTimeHms: string;
    messageId: number;
    messageLogId: number;
    userId: number;
    userRank: number;
    isRead: number;
    title: string;
    content: string;
    link: object;
    addTime: string;
}

export interface UserMsgFilterResult extends BaseResponseListWrap<UserMsgFilterState[]> {
    // filter: UserMsgFilterParams;
}
