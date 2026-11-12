export type MerchantSubjectType = 1 | 2 | 3;

export interface MerchantApplyData {
    type?: MerchantSubjectType;
    shopTitle?: string;
    baseData: { type: MerchantSubjectType; [key: string]: unknown };
    merchantData: {
        bankDeposit?: string;
        bankBranch?: string;
        bankCard?: string;
        accountOpeningPermit?: { picUrl: string; [key: string]: unknown }[] | null;
        [key: string]: unknown;
    };
    shopData: { shopTitle?: string; [key: string]: unknown };
    [key: string]: unknown;
}
