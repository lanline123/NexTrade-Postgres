import type { MerchantApplyData } from "~/types/merchant/apply";
import { buildMerchantApplyPayload } from "~/utils/merchantApply.mjs";

export const applyMerchant = (data: MerchantApplyData) => {
    return asyncRequest({
        url: "merchant/merchant/apply",
        method: "post",
        data: buildMerchantApplyPayload(data)
    });
};

export const getMerchantInfo = (id: any) => {
    return asyncRequest<any>({
        url: "merchant/merchant/applyDetail",
        method: "get",
        params: { id }
    });
};

export const getMyMerchant = () => {
    return asyncRequest<any>({
        url: "merchant/merchant/myApply",
        method: "get"
    });
};

export const getApplyShopAgreement = () => {
    return asyncRequest<any>({
        url: "merchant/merchant/applyShopAgreement",
        method: "get"
    });
};
