import { useConfigStore } from "@/store/config";
import { useUserStore } from "@/store/user";

/**
 * 是否需要显示 B2B 登录或实名认证后查看价格的提示。
 *
 * 价格查看门控双开关（2026-11-21 拆分）：isIdentity（认证后可查）
 * 开启时，未登录或未认证都提示；isLogin（登录后可查）单独开启时，
 * 仅未登录提示，任意已登录用户不提示。两开关都关恒不提示。
 */
export const isShowB2bPriceText = () => {
    const { isIdentity, isLogin } = useConfigStore.getState();
    const { userInfo } = useUserStore.getState();
    if (isIdentity === 1) {
        return Object.keys(userInfo).length === 0 || !userInfo.isCompanyAuth;
    }
    if (isLogin === 1) {
        return Object.keys(userInfo).length === 0;
    }
    return false;
};
