'use client';

/**
 * 首访定位匹配：首次进入站点（无 locale / currentCurrencyData Cookie、
 * 且未做过定位匹配）时，按 IP 定位的国家匹配语言与货币。
 *
 * - 数据源 ipapi.co（返回 country_code / currency / languages），3 秒超时、失败静默；
 * - 语言只在后台已启用的语言列表内选，带独立域名的语言不自动跳；
 * - 货币优先用定位返回的 ISO 币种在后台货币列表里找，找不到再按语言联动表；
 * - 命中与当前默认不同才写 Cookie 并整页刷新一次；无论结果如何都写
 *   geoLocaleChecked 标记，保证只在首访跑一次，之后以用户手动选择为准。
 */
import { useEffect } from 'react';
import { normalizeLocale, writeLocaleCookie } from '@/i18n';
import { readHeaderBootstrap } from '@/lib/header-bootstrap';
import { currencyCodeForLocale, findCurrencyByCode, localeForCountry } from '@/lib/locale-geo';

const MARKER_COOKIE = 'geoLocaleChecked';
const CURRENT_LANGUAGE_COOKIE = 'currentLanguage';

function readCookie(name: string): string {
  if (typeof document === 'undefined') return '';
  const escaped = name.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  const match = document.cookie.match(new RegExp(`(?:^|; )${escaped}=([^;]*)`));
  return match ? match[1] : '';
}

function writePlainCookie(name: string, value: string): void {
  document.cookie = `${name}=${encodeURIComponent(value)}; path=/; max-age=31536000; SameSite=Lax`;
}

interface GeoResponse {
  country_code?: string;
  currency?: string;
  languages?: string;
}

export default function PcGeoLocaleInit() {
  useEffect(() => {
    if (readCookie('locale') || readCookie('currentCurrencyData') || readCookie(MARKER_COOKIE)) return;
    // 先落标记：StrictMode 双跑、刷新重入都只执行一次
    writePlainCookie(MARKER_COOKIE, '1');

    const ctrl = new AbortController();
    const timer = window.setTimeout(() => ctrl.abort(), 3000);

    const run = async () => {
      let data: GeoResponse | null = null;
      try {
        const response = await fetch('https://ipapi.co/json/', { signal: ctrl.signal });
        if (response.ok) data = (await response.json()) as GeoResponse;
      } catch {
        data = null;
      }
      window.clearTimeout(timer);
      if (!data) return;

      const bootstrap = readHeaderBootstrap();
      let locales = (bootstrap?.locales || []).filter(
        (item) => item.isEnabled === undefined || Number(item.isEnabled) !== 0,
      );
      let currencies = bootstrap?.currencies || [];
      if (!locales.length) {
        try {
          const json = await (await fetch('/api/common/i18n/getLocales')).json();
          locales = (Array.isArray(json?.data) ? json.data : []).filter(
            (item: { isEnabled?: number }) => item.isEnabled === undefined || Number(item.isEnabled) !== 0,
          );
        } catch {
          locales = [];
        }
      }
      if (!currencies.length) {
        try {
          const json = await (await fetch('/api/common/currency/getCurrency')).json();
          currencies = Array.isArray(json?.data) ? json.data : [];
        } catch {
          currencies = [];
        }
      }
      if (!locales.length && !currencies.length) return;

      // 语言候选：国家映射优先，其次定位返回的 languages 首项
      let candidate = localeForCountry(data.country_code);
      if (!candidate && data.languages) {
        candidate = normalizeLocale(String(data.languages).split(',')[0]);
      }
      let targetLocale: (typeof locales)[number] | undefined;
      if (candidate) {
        const normalizedCandidate = normalizeLocale(candidate);
        targetLocale = locales.find((item) => normalizeLocale(item.localeCode) === normalizedCandidate);
        if (!targetLocale && normalizedCandidate !== 'zh-Hant') {
          const lang = normalizedCandidate.split('-')[0];
          targetLocale = locales.find((item) => normalizeLocale(item.localeCode).split('-')[0] === lang);
        }
      }

      // 货币候选：定位币种优先，其次语言联动表
      const currencyIso =
        String(data.currency || '').toUpperCase() ||
        (targetLocale ? currencyCodeForLocale(targetLocale.localeCode) : '');
      const targetCurrency = findCurrencyByCode(currencies, currencyIso);

      const defaultLocale = locales.find((item) => Number(item.isDefault) === 1) || locales[0];
      const defaultCurrency = currencies.find((item) => Number(item.isDefault) === 1) || currencies[0];

      let changed = false;
      if (
        targetLocale &&
        !String(targetLocale.domain || '').trim() &&
        defaultLocale &&
        normalizeLocale(targetLocale.localeCode) !== normalizeLocale(defaultLocale.localeCode)
      ) {
        writeLocaleCookie(normalizeLocale(targetLocale.localeCode));
        writePlainCookie(CURRENT_LANGUAGE_COOKIE, targetLocale.language || targetLocale.localeCode.toUpperCase());
        changed = true;
      }
      if (targetCurrency && defaultCurrency && targetCurrency.id !== defaultCurrency.id) {
        const value = encodeURIComponent(JSON.stringify(targetCurrency));
        document.cookie = `currentCurrencyData=${value}; path=/; max-age=31536000; SameSite=Lax`;
        changed = true;
      }
      if (changed) window.location.reload();
    };

    void run();
    return () => {
      window.clearTimeout(timer);
      ctrl.abort();
    };
  }, []);

  return null;
}
