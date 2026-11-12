export interface PremiumMemberCheckoutDraft {
    cardId: number;
    tierId: number;
    buyType: 1 | 2 | 3;
    profile?: Record<string, string>;
    agreementAccepted?: boolean;
}

const storageKey = (token: string) => `premium-member-checkout:${token}`;

export const savePremiumMemberCheckoutDraft = (draft: PremiumMemberCheckoutDraft) => {
    const token = `${Date.now()}-${Math.random().toString(36).slice(2, 10)}`;
    if (import.meta.client) sessionStorage.setItem(storageKey(token), JSON.stringify(draft));
    return token;
};

export const loadPremiumMemberCheckoutDraft = (token: string) => {
    if (!import.meta.client || !token) return undefined;
    try {
        const value = JSON.parse(sessionStorage.getItem(storageKey(token)) || "null");
        return value && Number(value.cardId) > 0 && Number(value.tierId) > 0 ? (value as PremiumMemberCheckoutDraft) : undefined;
    } catch {
        return undefined;
    }
};

export const removePremiumMemberCheckoutDraft = (token: string) => {
    if (import.meta.client && token) sessionStorage.removeItem(storageKey(token));
};
