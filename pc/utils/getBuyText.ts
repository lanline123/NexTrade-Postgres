import { useCommonStore } from "~/store/common";
import { useUserStore } from "~/store/user";
import { isB2B } from "~/utils/util";

export default function getBuyText(type = 1) {
    const commonStore = useCommonStore();
    const userStore = useUserStore();
    let str = "";
    if (type === 1) {
        str = "加入购物车";
    } else if (type === 5 || type === 6) {
        str = "立即预约";
    } else {
        str = "立即购买";
    }

    if (isB2B() && commonStore.isIdentity === 1) {
        if (!userStore.isAuthenticated) {
            str = "登录后采购";
        } else if (!userStore.userInfo.isCompanyAuth) {
            str = "实名后采购";
        }
    }
    return str;
}
