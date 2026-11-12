import { asyncRequest } from "~/utils/request";
import type { CommentFilterParams } from "~/types/user/comment.d";
// 获取评价列表数字角标
export const getCommentSubNum = () => {
    return asyncRequest<any>({
        url: "user/comment/subNum",
        method: "get"
    });
};

// 获取带晒单评价列表
export const getShowedList = (params: CommentFilterParams) => {
    return asyncRequest({
        url: "user/comment/showedList?isShowed=0",
        method: "get",
        params
    });
};

// 获取已评价列表
export const getCommentList = (params: CommentFilterParams) => {
    return asyncRequest({
        url: "user/comment/list",
        method: "get",
        params
    });
};

// 上传评论
export const updateCommentData = (data: object) => {
    return asyncRequest({
        url: "user/comment/evaluate",
        method: "post",
        data
    });
};

//获取评价详情
export const getCommentData = (params: object) => {
    return asyncRequest<any>({
        url: "user/comment/detail",
        method: "get",
        params
    });
};
