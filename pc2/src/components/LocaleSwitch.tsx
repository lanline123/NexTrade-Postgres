'use client';

import { Select } from '@douyinfe/semi-ui';
import { useTranslation } from 'react-i18next';
import { LOCALE_COOKIE, SUPPORTED_LOCALES, type Locale } from '@/i18n';

const LABELS: Record<Locale, string> = {
    zh_cn: '简体中文',
    en_us: 'English'
};

export default function LocaleSwitch() {
    const { i18n } = useTranslation();
    const current: Locale = i18n.language === 'en_us' ? 'en_us' : 'zh_cn';

    return (
        <Select
            value={current}
            size="small"
            style={{ width: 130 }}
            onChange={(next) => {
                const locale = next as Locale;
                document.cookie = `${LOCALE_COOKIE}=${locale}; path=/; max-age=31536000`;
                void i18n.changeLanguage(locale);
            }}
        >
            {SUPPORTED_LOCALES.map((locale) => (
                <Select.Option key={locale} value={locale}>
                    {LABELS[locale]}
                </Select.Option>
            ))}
        </Select>
    );
}
