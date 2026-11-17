/**
 * 网站字体注册表（前端副本，与后端 services/system/font_settings.rs 的
 * 白名单、pc-semi/src/lib/font-settings.ts 三方同表）。
 *
 * 配置（config code `fontSettings`）只存 fontId 引用；每款字体的
 * font-family 名与切片 CSS 地址在本表登记。字体文件自托管于主站
 * 静态目录（battery.recy.cc/fonts/<fontId>/font.css，unicode-range
 * 分片，浏览器按实际用字下载）。
 */

export interface FontDef {
  id: string;
  /** 设置页显示名 */
  name: string;
  /** CSS font-family 名（system 为空，表示不引外部字体、用系统栈） */
  family: string;
  /** 切片 CSS 地址（system 为空） */
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

/** 系统字体栈（与 admin styles.css 原 body 字体栈一致，作所有字体栈的最终兜底）。 */
export const SYSTEM_FONT_STACK =
  '-apple-system, BlinkMacSystemFont, "Segoe UI", "PingFang SC", "Hiragino Sans GB", "Microsoft YaHei", sans-serif';

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

/** fontId → FontDef（含 customFonts 逃生舱）。未知 id 回落默认字体。 */
export function resolveFont(
  fontId: string,
  customFonts: CustomFont[] = [],
): FontDef {
  const custom = customFonts.find((f) => f.id === fontId);
  if (custom) {
    return { id: custom.id, name: custom.name, family: custom.family, cssUrl: custom.cssUrl };
  }
  return FONT_MAP[fontId] || FONT_MAP[DEFAULT_FONT_ID];
}

/**
 * 拼装生效字体栈：所选字体 → 全站默认字体（普惠体，去重）→ 系统栈。
 * CSS 逐字回退：混排文本中，所选字体缺的字形自动落到普惠体、再落系统栈。
 */
export function buildFontStack(
  fontId: string,
  customFonts: CustomFont[] = [],
): string {
  const parts: string[] = [];
  const push = (family: string) => {
    if (family && !parts.includes(`"${family}"`)) parts.push(`"${family}"`);
  };
  push(resolveFont(fontId, customFonts).family);
  push(resolveFont(DEFAULT_FONT_ID).family);
  parts.push(SYSTEM_FONT_STACK);
  return parts.join(', ');
}

/** 需要加载的字体 CSS（所选 + 默认字体，去重后的地址列表）。 */
export function fontCssUrls(
  fontId: string,
  customFonts: CustomFont[] = [],
): string[] {
  const urls: string[] = [];
  for (const id of [fontId, DEFAULT_FONT_ID]) {
    const def = resolveFont(id, customFonts);
    if (def.cssUrl && !urls.includes(def.cssUrl)) urls.push(def.cssUrl);
  }
  return urls;
}

const loadedCss = new Set<string>();

/** 动态插入字体 CSS（已加载过的地址不重复插入）。 */
export function ensureFontCss(urls: string[]): void {
  for (const url of urls) {
    if (!url || loadedCss.has(url)) continue;
    loadedCss.add(url);
    const link = document.createElement('link');
    link.rel = 'stylesheet';
    link.href = url;
    link.dataset.fontCss = '1';
    document.head.appendChild(link);
  }
}

/** 把字体栈写到 --font-family-base（admin styles.css 的 body 已改走该变量）。 */
export function applyAdminFont(cfg: FontSettingsConfig): void {
  const fontId = cfg?.admin?.fontId || DEFAULT_FONT_ID;
  ensureFontCss(fontCssUrls(fontId, cfg?.customFonts || []));
  document.documentElement.style.setProperty(
    '--font-family-base',
    buildFontStack(fontId, cfg?.customFonts || []),
  );
}
