/**
 * 语言 ↔ 货币联动与 IP 定位匹配（纯函数，客户端组件共用）。
 *
 * - currencyCodeForLocale：切换语言时货币跟随的目标币种（ISO 代码）。
 * - localeForCountry：首访按 IP 定位的国家/地区码选首选语言。
 * 币种的实际条目（符号/汇率）以后台 getCurrency 列表为准，这里只做代码级映射，
 * 列表里没有对应币种时不强行切换（findCurrencyByCode 返回 undefined）。
 */
import { normalizeLocale } from '@/i18n/locale';
import { currencyCodeFrom } from '@/lib/currency';

const LOCALE_CURRENCY: Record<string, string> = {
  'zh-CN': 'CNY',
  'zh-Hant': 'CNY',
  'en-US': 'USD',
  ja: 'JPY',
  ko: 'KRW',
  es: 'EUR',
  fr: 'EUR',
  de: 'EUR',
  it: 'EUR',
  pt: 'EUR',
  ru: 'RUB',
};

export function currencyCodeForLocale(locale: string): string {
  return LOCALE_CURRENCY[normalizeLocale(locale)] || '';
}

export function findCurrencyByCode<T extends { name?: string; symbol?: string }>(
  list: T[],
  code: string,
): T | undefined {
  if (!code) return undefined;
  return list.find((item) => currencyCodeFrom(item.name, item.symbol) === code);
}

const COUNTRY_LOCALE: Record<string, string> = {
  CN: 'zh-CN',
  TW: 'zh-Hant',
  HK: 'zh-Hant',
  MO: 'zh-Hant',
  US: 'en-US',
  GB: 'en-US',
  AU: 'en-US',
  CA: 'en-US',
  NZ: 'en-US',
  SG: 'en-US',
  IE: 'en-US',
  JP: 'ja',
  KR: 'ko',
  ES: 'es',
  MX: 'es',
  AR: 'es',
  CO: 'es',
  CL: 'es',
  PE: 'es',
  FR: 'fr',
  BE: 'fr',
  RU: 'ru',
  BY: 'ru',
  KZ: 'ru',
  DE: 'de',
  AT: 'de',
  CH: 'de',
};

export function localeForCountry(countryCode?: string | null): string {
  return COUNTRY_LOCALE[String(countryCode || '').toUpperCase()] || '';
}
