import { asyncRequest } from "~/utils/request";
import type { FeedbackFilterParams, FeedbackFilterResult } from "~/types/user/feedback.d";

// 获取留言
export const getFeedbackList = (params: FeedbackFilterParams) => {
    return asyncRequest<FeedbackFilterResult>({
        url: "user/feedback/list",
        method: "get",
        params,
        noSkipLogin: true
    });
};
// 保存
export const updateFeedback = (data: object) => {
    return asyncRequest({
        url: "user/feedback/submit",
        method: "post",
        data
    });
};
