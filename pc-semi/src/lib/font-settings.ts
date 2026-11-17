/**
 * 全站字体（后台「网站字体设置」fontSettings 配置）。
 *
 * 配置由 layout 服务端预取（common/config/fontSettings）→ 内联 bootstrap，
 * 与 buyStyle / priceDisplay 同一条链路。本模块提供：
 *  - 字体注册表（与后端 services/system/font_settings.rs 白名单、
 *    admin-semi/src/fontRegistry.ts 三方同表）：配置只存 fontId 引用，
 *    family / 切片 CSS 地址在本表登记；字体文件自托管于主站
 *    /fonts/<fontId>/font.css（unicode-range 分片，按用字下载）。
 *  - 按 locale 解析生效字体的纯函数（服务端 layout 注入 <link> 与
 *    --font-family-base 变量时使用）。
 *
 * 生效字体栈 = 当前语言所选字体 → 全站默认字体（普惠体）→ 原系统栈，
 * CSS 逐字回退：混排文本中拉丁走所选字体、汉字自动落普惠体。
 */

export interface FontDef {
  id: string;
  name: string;
  /** CSS font-family 名；system 为空（不引外部字体）。 */
  family: string;
  /** 切片 CSS 地址；system 为空。 */
  cssUrl: string;
}

const FONT_BASE = 'https://battery.recy.cc/fonts';

export const FONT_REGISTRY: FontDef[] = [
  { id: 'alibaba-puhuiti', name: '阿里巴巴普惠体 3.0', family: 'Alibaba PuHuiTi 3', cssUrl: `${FONT_BASE}/alibaba-puhuiti/font.css` },
  { id: 'noto-sans-sc', name: '思源黑体（简体）', family: 'Noto Sans SC', cssUrl: `${FONT_BASE}/noto-sans-sc/font.css` },
  { id: 'noto-sans-tc', name: '思源黑体（繁体）', family: 'Noto Sans TC', cssUrl: `${FONT_BASE}/noto-sans-tc/font.css` },
  { id: 'noto-sans-jp', name: '思源黑体（日文）', family: 'Noto Sans JP', cssUrl: `${FONT_BASE}/noto-sans-jp/font.css` },
  { id: 'noto-sans-kr', name: '思源黑体（韩文）', family: 'Noto Sans KR', cssUrl: `${FONT_BASE}/noto-sans-kr/font.css` },
  { id: 'inter', name: 'Inter（拉丁）', family: 'Inter', cssUrl: `${FONT_BASE}/inter/font.css` },
  { id: 'roboto', name: 'Roboto（拉丁/西里尔）', family: 'Roboto', cssUrl: `${FONT_BASE}/roboto/font.css` },
  { id: 'system', name: '系统默认', family: '', cssUrl: '' },
];

export const FONT_MAP: Record<string, FontDef> = Object.fromEntries(
  FONT_REGISTRY.map((f) => [f.id, f]),
);

/** 全站默认字体 id（用户指定：阿里巴巴普惠体）。 */
export const DEFAULT_FONT_ID = 'alibaba-puhuiti';

/** 原 body 字体栈（globals.scss 旧版 global.less 基线），作最终兜底。 */
export const SYSTEM_FONT_STACK =
  'PingFang SC, Microsoft Yahei, Avenir, Segoe UI, Hiragino Sans GB, STHeiti, "Microsoft Sans Serif", WenQuanYi Micro Hei, sans-serif';

export interface CustomFont {
  id: string;
  name: string;
  family: string;
  cssUrl: string;
}

export interface FontSettingsConfig {
  version: number;
  storefront: {
    default: { fontId: string };
    locales: Record<string, { fontId: string }>;
  };
  admin: { fontId: string };
  customFonts: CustomFont[];
}

export const DEFAULT_FONT_SETTINGS: FontSettingsConfig = {
  version: 1,
  storefront: {
    default: { fontId: DEFAULT_FONT_ID },
    locales: {
      'zh-CN': { fontId: 'alibaba-puhuiti' },
      'zh-Hant': { fontId: 'alibaba-puhuiti' },
      'en-US': { fontId: 'inter' },
      fr: { fontId: 'inter' },
      ko: { fontId: 'noto-sans-kr' },
      ja: { fontId: 'noto-sans-jp' },
      ru: { fontId: 'roboto' },
    },
  },
  admin: { fontId: DEFAULT_FONT_ID },
  customFonts: [],
};

/** 把 bootstrap 的原始 JSON 规整为 FontSettingsConfig（缺失/非法 ⇒ 内置默认）。 */
export function normalizeFontSettings(raw: Record<string, any> | null | undefined): FontSettingsConfig {
  if (!raw || typeof raw !== 'object') return DEFAULT_FONT_SETTINGS;
  const sf = (raw.storefront || {}) as Record<string, any>;
  const locales: Record<string, { fontId: string }> = {};
  if (sf.locales && typeof sf.locales === 'object') {
    for (const [k, v] of Object.entries(sf.locales as Record<string, any>)) {
      if (v && typeof v === 'object' && typeof v.fontId === 'string') {
        locales[k] = { fontId: v.fontId };
      }
    }
  }
  return {
    version: 1,
    storefront: {
      default: { fontId: typeof sf.default?.fontId === 'string' ? sf.default.fontId : DEFAULT_FONT_ID },
      locales,
    },
    admin: { fontId: typeof raw.admin?.fontId === 'string' ? raw.admin.fontId : DEFAULT_FONT_ID },
    customFonts: Array.isArray(raw.customFonts) ? raw.customFonts : [],
  };
}

export function resolveFont(fontId: string, customFonts: CustomFont[] = []): FontDef {
  const custom = customFonts.find((f) => f.id === fontId);
  if (custom) {
    return { id: custom.id, name: custom.name, family: custom.family, cssUrl: custom.cssUrl };
  }
  return FONT_MAP[fontId] || FONT_MAP[DEFAULT_FONT_ID];
}

export interface ResolvedFont {
  fontId: string;
  /** --font-family-base 的值（完整字体栈）。 */
  stack: string;
  /** 需要 <link> 引入的切片 CSS（所选 + 默认字体，去重）。 */
  cssUrls: string[];
}

/** 按当前 locale 解析生效字体：locales[locale] → storefront.default → 内置默认。 */
export function resolveFontForLocale(
  raw: Record<string, any> | null | undefined,
  locale: string,
): ResolvedFont {
  const cfg = normalizeFontSettings(raw);
  const fontId =
    cfg.storefront.locales[locale]?.fontId || cfg.storefront.default.fontId || DEFAULT_FONT_ID;
  const parts: string[] = [];
  const push = (family: string) => {
    if (family && !parts.includes(`"${family}"`)) parts.push(`"${family}"`);
  };
  push(resolveFont(fontId, cfg.customFonts).family);
  push(resolveFont(DEFAULT_FONT_ID, cfg.customFonts).family);
  parts.push(SYSTEM_FONT_STACK);
  const cssUrls: string[] = [];
  for (const id of [fontId, DEFAULT_FONT_ID]) {
    const def = resolveFont(id, cfg.customFonts);
    if (def.cssUrl && !cssUrls.includes(def.cssUrl)) cssUrls.push(def.cssUrl);
  }
  return { fontId, stack: parts.join(', '), cssUrls };
}
