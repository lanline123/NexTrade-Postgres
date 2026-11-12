import type { Metadata, Viewport } from 'next';
import type { ReactNode } from 'react';
import { cookies } from 'next/headers';
import I18nProvider from '@/i18n/I18nProvider';
import { LOCALE_COOKIE, normalizeLocale } from '@/i18n';
// Semi UI 的 dist/css 未在 package.json exports 中导出，直接 import 会编译失败。
// 与 pc-semi 一致：把 semi.min.css 落到本地再引入。
import './semi.css';
import './globals.css';

export const metadata: Metadata = {
    title: 'NexTrade PC · React + Semi',
    description: 'pc React + Semi Design SSR 重构版'
};

export const viewport: Viewport = {
    width: 'device-width',
    initialScale: 1
};

export default function RootLayout({ children }: { children: ReactNode }) {
    // 服务端读 cookie，让 SSR 首屏就是正确语言（react-i18next，不用 Vue 的 $t）。
    const locale = normalizeLocale(cookies().get(LOCALE_COOKIE)?.value);
    return (
        <html lang={locale}>
            <body>
                <I18nProvider locale={locale}>{children}</I18nProvider>
            </body>
        </html>
    );
}
