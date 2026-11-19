/**
 * 语言标识的纯函数工具（server / client 共用，不能带 'use client'）。
 *
 * 后台 localeCode 是短码：zh / zh-Hant / en / ko / ja / es / fr / ru。
 * 前端内部统一用 BCP47 形态（zh-CN / en-US / zh-Hant / ko ...），
 * 调用后台翻译接口时再映射回短码（接口不认 en-US / zh-CN）。
 */

export const LOCALE_COOKIE = 'locale';

export function normalizeLocale(locale?: string | null): string {
  const value = String(locale || '').trim();
  if (!value) return 'zh-CN';
  const lower = value.toLowerCase();
  if (lower.startsWith('en')) return 'en-US';
  if (lower.startsWith('zh-hant') || lower.startsWith('zh-tw') || lower.startsWith('zh-hk')) {
    return 'zh-Hant';
  }
  if (lower.startsWith('zh')) return 'zh-CN';
  // 其他语言原样保留（ko / ja / es / fr / ru ...）
  return value;
}

/** 内部语言标识 → 后台 getLocaleTranslations 的 localeCode。 */
export function toApiLocale(locale: string): string {
  const value = normalizeLocale(locale);
  if (value === 'zh-CN') return 'zh';
  if (value === 'en-US') return 'en';
  return value;
}

/**
 * URL 语言前缀路由（SEO hreflang 用）。
 *
 * 默认语言 zh-CN 不带前缀（与历史 URL 完全一致）；其余已落地的语言在
 * LOCALE_URL_PREFIXES 里登记「BCP47 → 路径首段」。新增语言只需在此登记，
 * middleware / canonical / hreflang / sitemap / 语言切换器都读同一份表。
 * 注意：这里登记的语言必须已有内置或后台翻译资源，否则前缀页只是中文换皮。
 */
export const DEFAULT_LOCALE = 'zh-CN';

export const LOCALE_URL_PREFIXES: Record<string, string> = {
  'en-US': 'en',
};

/** 某语言的 URL 前缀（默认语言与未登记语言为 ''，即无前缀）。 */
export function urlPrefixForLocale(locale?: string | null): string {
  return LOCALE_URL_PREFIXES[normalizeLocale(locale)] || '';
}

/** URL 首段 → 语言（不是已登记前缀时返回 null）。 */
export function localeForUrlPrefix(segment?: string | null): string | null {
  const seg = String(segment || '').trim().toLowerCase();
  if (!seg) return null;
  for (const [locale, prefix] of Object.entries(LOCALE_URL_PREFIXES)) {
    if (prefix === seg) return locale;
  }
  return null;
}

/**
 * 拆解路径的语言前缀：命中已登记前缀时返回 { locale, path（剥离后） }，
 * 否则 locale=null、path 为原路径。只认首段精确匹配（/en、/en/...），
 * /energy 这类真实路径不受影响。
 */
export function splitLocalePrefix(pathname: string): { locale: string | null; path: string } {
  const path = pathname.startsWith('/') ? pathname : `/${pathname}`;
  const first = path.split('/')[1] || '';
  const locale = localeForUrlPrefix(first);
  if (!locale) return { locale: null, path };
  const rest = path.slice(first.length + 1);
  return { locale, path: rest === '' ? '/' : rest };
}

/**
 * 给无前缀路径拼上指定语言的前缀。path 可带 query（?a=b），前缀只加在路径段前。
 * 默认语言/未登记语言原样返回。
 */
export function localizedPath(path: string, locale?: string | null): string {
  const prefix = urlPrefixForLocale(locale);
  if (!prefix) return path;
  const queryIndex = path.indexOf('?');
  const pathname = queryIndex >= 0 ? path.slice(0, queryIndex) : path;
  const query = queryIndex >= 0 ? path.slice(queryIndex) : '';
  const clean = pathname.startsWith('/') ? pathname : `/${pathname}`;
  return `/${prefix}${clean === '/' ? '' : clean}${query}` || `/${prefix}`;
}

/**
 * 按当前语言生成 Next metadata 的 alternates（canonical 自指当前语言版本，
 * languages 含全部已登记语言 + 默认语言 + x-default）。
 * path 必须是无前缀的内部路径（可带 query），返回值均为相对路径，
 * 由 metadataBase 解析为绝对地址。
 */
export function buildLocaleAlternates(
  path: string,
  currentLocale?: string | null,
): { canonical: string; languages: Record<string, string> } {
  const languages: Record<string, string> = {
    [DEFAULT_LOCALE]: path,
  };
  for (const [locale, prefix] of Object.entries(LOCALE_URL_PREFIXES)) {
    // hreflang 值用前缀短码（en），与搜索引擎惯例一致
    languages[prefix] = localizedPath(path, locale);
  }
  languages['x-default'] = path;
  return {
    canonical: localizedPath(path, currentLocale),
    languages,
  };
}
