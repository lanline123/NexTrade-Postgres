/** Normalize API details before binding upload controls and numeric subject selectors. */
export function normalizeMerchantApplication(application) {
    const type = Number(application.baseData?.type ?? application.type);
    return {
        ...application,
        type,
        baseData: { ...application.baseData, type },
        merchantData: { ...application.merchantData, accountOpeningPermit: application.merchantData?.accountOpeningPermit ?? [] },
        shopData: { ...application.shopData }
    };
}

/** 切换到个体工商户时仅清理隐藏的开户许可证，保留银行卡号。 */
export function selectMerchantType(application, type) {
    const result = normalizeMerchantApplication({ ...application, baseData: { ...application.baseData, type } });
    if (result.type === 3) {
        for (const field of ['bankCardTemp', 'accountOpeningPermit', 'accountOpeningPermitTemp']) {
            delete result.merchantData[field];
        }
    }
    return result;
}

export function buildMerchantApplyPayload(application) {
    const result = selectMerchantType(application, application.baseData?.type ?? application.type);
    return { ...result, shopTitle: result.shopData.shopTitle };
}

export function merchantReviewStep(type, status) {
    return (Number(type) === 1 ? 3 : 4) + (Number(status) === 10 ? 1 : 0);
}
