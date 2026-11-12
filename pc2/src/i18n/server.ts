import { cookies } from 'next/headers';
import { LOCALE_COOKIE, normalizeLocale, type Locale } from './index';

/** 服务端组件读语言 cookie，保证 SSR 首屏就是正确语言（避免 hydration 后跳变）。 */
export function getServerLocale(): Locale {
    return normalizeLocale(cookies().get(LOCALE_COOKIE)?.value);
}
