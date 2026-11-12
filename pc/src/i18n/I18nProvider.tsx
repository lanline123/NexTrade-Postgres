'use client';

import { useEffect, type ReactNode } from 'react';
import { I18nextProvider } from 'react-i18next';
import i18n, { type Locale } from './index';

export default function I18nProvider({ children, locale }: { children: ReactNode; locale: Locale }) {
    // 同步设置：服务端渲染也会执行组件体，保证 SSR 首屏与客户端 hydration 用同一语言，
    // 不放到 useEffect 里，否则英文用户会先看到中文再跳变。资源是内联的，切换即时生效。
    if (i18n.language !== locale) {
        void i18n.changeLanguage(locale);
    }

    useEffect(() => {
        document.documentElement.lang = locale;
    }, [locale]);

    return <I18nextProvider i18n={i18n}>{children}</I18nextProvider>;
}
