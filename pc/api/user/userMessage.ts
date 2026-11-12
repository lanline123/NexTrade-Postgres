import type { UserMsgFilterParams, UserMsgFilterResult } from "~/types/user/userMessage.d";
//获取站内信列表
export const getMessageList = (params: UserMsgFilterParams) => {
    return asyncRequest<UserMsgFilterResult>({
        url: "user/message/list",
        method: "get",
        params
    });
};

//站内信单个标记已读
export const addMessageRead = (id: number) => {
    return asyncRequest({
        url: "user/message/updateMessageRead",
        method: "post",
        data: { id }
    });
};

//站内信全部标记已读
export const addMessageAllRead = () => {
    return asyncRequest({
        url: "user/message/updateAllRead",
        method: "post"
    });
};

//删除站内信
export const delMessage = (data: Object) => {
    return asyncRequest({
        url: "user/message/del",
        method: "post",
        data
    });
};
