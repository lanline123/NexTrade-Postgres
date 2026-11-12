// 跨境独有：Cookie/GDPR 同意状态管理
import { computed } from "vue";
import { sharedCookieDomain } from "~/utils/cookieDomain";

export type ConsentType = "functional" | "analytics" | "marketing";

export interface ConsentDetail {
    necessary: true;
    functional: boolean;
    analytics: boolean;
    marketing: boolean;
    policyVersion?: string;
    updatedAt?: number;
}

/**
 * Cookie 同意策略版本号
 * 当文案 / 分类变更，升版本号将会让历史用户重新弹窗同意
 */
export const PRIVACY_POLICY_VERSION = "v1";

const CONSENT_COOKIE_KEY = "privacyConsent";
const CONSENT_DETAIL_KEY = "privacyConsentDetail";
const COOKIE_MAX_AGE = 60 * 60 * 24 * 365; // 1 年

export const useConsent = () => {
    // 本地句柄用于保持响应式状态；共享 Domain 的持久化写入统一走 writeConsentCookies
    const consentCookie = useCookie<string | null>(CONSENT_COOKIE_KEY);

    const detail = useCookie<ConsentDetail | null>(CONSENT_DETAIL_KEY);

    const writeConsentCookies = (mode: string, value: ConsentDetail) => {
        const options = { domain: sharedCookieDomain(), maxAge: COOKIE_MAX_AGE, sameSite: "lax" as const };
        useCookie<ConsentDetail | null>(CONSENT_DETAIL_KEY, options).value = value;
        useCookie<string | null>(CONSENT_COOKIE_KEY, options).value = mode;
    };

    // 是否需要重新弹窗：未决定，或政策版本已升级
    const needRedecide = computed(() => {
        if (!consentCookie.value) return true;
        if (!detail.value) return true;
        if (detail.value.policyVersion !== PRIVACY_POLICY_VERSION) return true;
        return false;
    });

    const hasConsent = (type: ConsentType): boolean => {
        if (!detail.value) return false;
        return !!detail.value[type];
    };

    const showSettings = useState<boolean>("privacy:showSettings", () => false);

    const openSettings = () => {
        showSettings.value = true;
    };

    const closeSettings = () => {
        showSettings.value = false;
    };

    const writeDetail = (partial: Pick<ConsentDetail, "functional" | "analytics" | "marketing">, mode: string) => {
        const value: ConsentDetail = {
            necessary: true,
            functional: partial.functional,
            analytics: partial.analytics,
            marketing: partial.marketing,
            policyVersion: PRIVACY_POLICY_VERSION,
            updatedAt: Date.now()
        };
        detail.value = value;
        consentCookie.value = mode;
        writeConsentCookies(mode, value);
    };

    const acceptAll = () => writeDetail({ functional: true, analytics: true, marketing: true }, "all");

    const rejectAll = () => writeDetail({ functional: false, analytics: false, marketing: false }, "rejected");

    const saveCustom = (choices: Pick<ConsentDetail, "functional" | "analytics" | "marketing">) => writeDetail(choices, "custom");

    return {
        consentCookie,
        detail,
        needRedecide,
        hasConsent,
        showSettings,
        openSettings,
        closeSettings,
        acceptAll,
        rejectAll,
        saveCustom
    };
};

