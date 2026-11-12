import type { ProfileFormResult } from "~/types/user/profile.d";

/**
 * 获取用户资料
 * @description 获取当前登录用户的详细资料
 * @returns 用户资料
 */
export const getProfile = () => {
    return asyncRequest<ProfileFormResult>({
        url: "user/user/detail",
        method: "get",
        noSkipLogin: true
    });
};

/**
 * 更新用户资料
 * @description 修改用户的个人信息（昵称、头像等）
 * @param data - 要更新的资料数据
 * @returns 更新结果
 */
export const updateProfile = (data: object) => {
    return asyncRequest({
        url: "user/user/updateInformation",
        method: "post",
        data
    });
};
