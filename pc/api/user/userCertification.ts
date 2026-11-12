export const applyApply = (data: object) => {
    return asyncRequest({
        url: "user/company/apply",
        method: "post",
        data
    });
};

export const getApplyInfo = (id: any) => {
    return asyncRequest<any>({
        url: "user/company/detail",
        method: "get",
        params: { id }
    });
};

export const getMyApply = () => {
    return asyncRequest<any>({
        url: "user/company/myApply",
        method: "get"
    });
};
