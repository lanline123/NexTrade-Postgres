import i18next from 'i18next';
import { initReactI18next } from 'react-i18next';
import zhCN from './locales/zh_cn.json';
import enUS from './locales/en_us.json';

/**
 * React 侧多语言统一走 react-i18next（与 pc-semi 同栈）。
 * 语言资源从 Vue 版 i18n/locales/ 迁到 src/i18n/locales/，React 侧自持，
 * 后续删除 Vue 参照源码不受影响。key 沿用中文原文，迁移页面可直接 $t('中文') → t('中文')。
 */
export const SUPPORTED_LOCALES = ['zh_cn', 'en_us'] as const;
export type Locale = (typeof SUPPORTED_LOCALES)[number];
export const DEFAULT_LOCALE: Locale = 'zh_cn';
export const LOCALE_COOKIE = 'pc_locale';

export function normalizeLocale(value?: string | null): Locale {
    const normalized = (value || '').toLowerCase().replace(/-/g, '_');
    return normalized.startsWith('en') ? 'en_us' : 'zh_cn';
}

const resources = {
    zh_cn: { translation: zhCN },
    en_us: { translation: enUS }
};

// 资源内联，无需后端拉取；服务端与浏览器各初始化一次同一个单例。
if (!i18next.isInitialized) {
    void i18next.use(initReactI18next).init({
        resources,
        lng: DEFAULT_LOCALE,
        fallbackLng: DEFAULT_LOCALE,
        interpolation: { escapeValue: false },
        react: { useSuspense: false }
    });
}

export default i18next;
