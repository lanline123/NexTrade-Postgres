import { apiGet } from './api';

export interface MallSettings {
  clientDefaultUse?: unknown;
  pcDomain?: string;
  h5Domain?: string;
}

const IS_STORE = (import.meta.env.VITE_IS_STORE ?? '0') === '1';

let cache: MallSettings | null = null;
let inflight: Promise<MallSettings | null> | null = null;

export function setMallSettings(settings: MallSettings | null | undefined) {
  cache = settings || null;
}

export function getMallSettings(): MallSettings | null {
  return cache;
}

function trimDomain(value: string | undefined): string {
  return String(value || '').trim().replace(/\/+$/, '');
}

/**
 * 对齐旧版 MsgCount.vue 的 viewMallUrl 与 utils/format.ts 的 urlFormat/urlWapFormat：
 * 默认客户端为 H5 时用 h5Domain，否则用 pcDomain；店铺账号按 shopType 拼店铺地址。
 */
export function resolveMallUrl(settings: MallSettings | null | undefined = cache): string {
  const envUrl = import.meta.env.VITE_MALL_URL;
  if (envUrl) return envUrl;

  const origin = typeof window === 'undefined' ? '' : window.location.origin;
  const devBase = import.meta.env.DEV ? 'http://127.0.0.1:3004' : '';
  const pcDomain = trimDomain(settings?.pcDomain);
  const h5Domain = trimDomain(settings?.h5Domain);
  const useH5 = Number(settings?.clientDefaultUse) === 1;

  const shopId = Number(localStorage.getItem('shopId')) || 0;
  const shopType = Number(localStorage.getItem('shopType')) || 0;
  if (shopId > 0 && localStorage.getItem('adminType') === 'shop') {
    if (IS_STORE && shopType === 2) {
      return `${h5Domain || `${origin}/mobile`}/pages/store/index?id=${shopId}`;
    }
    if (shopType === 1) {
      return `${pcDomain}/shop/${shopId}`;
    }
  }

  if (useH5) {
    return `${h5Domain || (import.meta.env.DEV ? `${devBase}/mobile` : '')}/`;
  }
  return `${pcDomain || devBase}/`;
}

export async function loadMallSettings(): Promise<MallSettings | null> {
  if (cache) return cache;
  if (!inflight) {
    inflight = apiGet<MallSettings>('/setting/config/globalSettings')
      .then((result) => {
        cache = result || null;
        return cache;
      })
      .catch(() => null)
      .finally(() => {
        inflight = null;
      });
  }
  return inflight;
}
