import { useEffect, useState, type CSSProperties } from 'react';
import { apiGet } from '../api';
import { PcBuyButtonVisual, resolvePcBuyBtn, usePcGlobalBuyStyle, type PcBuyBtnPic } from '../pages/decoratePc/previews/shared/PcBuyButton';
import { BuyBtnTypeIcon } from '../pages/decorateEditorH5/shared/BuyButton';

/**
 * 购买按钮样式选择器（通用组件，2026-11-21 用户口径：装修编辑器里「购买按钮样式」
 * 不再用文字单选，直接显示按钮图标点选）。
 *
 * 形态与全局「购买按钮样式」页（pages/BuyStyleSettings.tsx）逐项对齐：
 * 横排卡片（面板内换行）、卡内上视觉下短标签、选中 = 主题色边框 + 浅底 +
 * 外圈同色描边、标签加粗。主题色取法与设置页同源：后台「主题风格」
 * themeStyle 的 --general（兜底 --main-bg，再兜底 #65c4aa），会话内只拉一次。
 *
 * 本组件只换控件形态：选项值、默认值、回写形状由调用方原样决定，点选时
 * 回传选项的原始数值，绝不做任何数据归一化。
 *
 * 视觉集（visualSet）：
 * - 'unified'：全站统一枚举（-1 跟随全局 / 0 不显示 / 1–18），视觉复用
 *   PcBuyButtonVisual（与设置页、前台同源）；-1 渲染当前全局样式的解析结果，
 *   10 渲染 customIcon（模块自带图标优先，由调用方传入）。
 * - 'h5BtnType'：H5 对象型 buyBtnStyle 的 btnType 枚举（1–5），视觉复用
 *   decorateEditorH5/shared/BuyButton 的同源图标（commonBuyButton 原版字形）；
 *   5 渲染 customIcon（模块 iconImage），未选图时为图片占位。
 *
 * 隐藏样式（全局配置 hiddenStyles，设置页「购买按钮样式」管理）：仅
 * 'unified' 集过滤——被隐藏的样式不再作为选项出现；但字段当前值若正是
 * 被隐藏的样式，该卡保留显示并标注「已隐藏」（存量数据不丢、仍可改选他样，
 * 改走后不可再选回）。'h5BtnType' 是另一套枚举，值域与统一枚举重叠但
 * 语义不同，绝不过滤。
 */

export interface BuyBtnStyleOption {
  value: number;
  /** 完整名称（title 提示） */
  label: string;
  /** 卡内短标签 */
  short: string;
}

/** 统一枚举 -1..18（标签与 BuyStyleSettings / pcModuleContract 口径一致）。 */
export const BUY_BTN_STYLE_OPTIONS_UNIFIED: BuyBtnStyleOption[] = [
  { value: -1, label: '跟随全局', short: '跟随全局' },
  { value: 0, label: '不显示购买按钮', short: '不显示' },
  { value: 1, label: '购物车（描边）', short: '购物车·描边' },
  { value: 2, label: '购物车（实心）', short: '购物车·实心' },
  { value: 3, label: '加号（描边）', short: '加号·描边' },
  { value: 4, label: '加号（实心）', short: '加号·实心' },
  { value: 5, label: '“购买”（描边）', short: '购买·描边' },
  { value: 6, label: '“购买”（实心）', short: '购买·实心' },
  { value: 7, label: '“马上抢”（描边）', short: '马上抢·描边' },
  { value: 8, label: '“马上抢”（实心）', short: '马上抢·实心' },
  { value: 9, label: '圆圈购物车', short: '圆圈购物车' },
  { value: 10, label: '自定义图片', short: '自定义图片' },
  { value: 11, label: '购物袋（描边）', short: '购物袋·描边' },
  { value: 12, label: '购物袋（实心）', short: '购物袋·实心' },
  { value: 13, label: '圆圈购物袋', short: '圆圈购物袋' },
  { value: 14, label: '圆圈加号', short: '圆圈加号' },
  { value: 15, label: '加购购物车', short: '加购购物车' },
  { value: 16, label: '闪电', short: '闪电' },
  { value: 17, label: '“立即购买”（描边）', short: '立即购买·描边' },
  { value: 18, label: '“立即购买”（实心）', short: '立即购买·实心' },
];

/** H5 数字型 buyBtnStyle 选项 0–8（值与存量数据逐字一致，仅形态换可视化）。 */
export const H5_BUY_BTN_STYLE_OPTIONS: BuyBtnStyleOption[] = BUY_BTN_STYLE_OPTIONS_UNIFIED.filter(
  (o) => o.value >= 0 && o.value <= 8,
);

/** H5 对象型 buyBtnStyle.btnType 选项 1–5（值与原版 buyBtn/Edit.vue 一致）。 */
export const H5_BUY_BTN_TYPE_OPTIONS: BuyBtnStyleOption[] = [
  { value: 1, label: '购物车图标', short: '购物车' },
  { value: 2, label: '购物篮图标', short: '购物篮' },
  { value: 3, label: '加号图标', short: '加号' },
  { value: 4, label: '减号图标', short: '减号' },
  { value: 5, label: '自定义图片', short: '自定义图片' },
];

/** PC 主题色兜底（与 BuyStyleSettings、编辑器预览兜底一致）。 */
const FALLBACK_THEME_COLOR = '#65c4aa';

/** 与 BuyStyleSettings.pickPcThemeColor 同口径（themeStyle 可为字符串化 JSON，危险字符弃用）。 */
function pickThemeColor(data: { themeStyle?: string | Record<string, any> } | null | undefined): string {
  try {
    const parsed = typeof data?.themeStyle === 'string' ? JSON.parse(data.themeStyle) : data?.themeStyle;
    if (!parsed || typeof parsed !== 'object') return FALLBACK_THEME_COLOR;
    for (const key of ['--general', '--main-bg']) {
      const raw = (parsed as Record<string, unknown>)[key];
      if (typeof raw !== 'string' && typeof raw !== 'number') continue;
      const text = String(raw).trim();
      if (!text || /[;{}<>\\]/.test(text)) continue;
      if (/^#(?:[0-9a-f]{3}|[0-9a-f]{6}|[0-9a-f]{8})$/i.test(text)) return text;
    }
  } catch {
    /* 解析失败走兜底色 */
  }
  return FALLBACK_THEME_COLOR;
}

/* ---------- 主题色懒加载（整个后台会话只拉一次，同 PcBuyButton 全局配置的缓存法） ---------- */

let themeCache: string | null = null;
let themePromise: Promise<void> | null = null;
const themeListeners = new Set<() => void>();

function loadThemeColor(): void {
  if (themeCache || themePromise) return;
  themePromise = apiGet<{ themeStyle?: string | Record<string, any> }>('/setting/config/themeStyleSettings')
    .then((data) => {
      themeCache = pickThemeColor(data);
    })
    .catch(() => {
      themeCache = FALLBACK_THEME_COLOR;
    })
    .finally(() => {
      themeListeners.forEach((fn) => fn());
      themeListeners.clear();
    });
}

/** 当前 PC 主题色（未加载完成时先返回兜底色，加载后自动重渲）。 */
export function useBuyStyleThemeColor(): string {
  const [, setTick] = useState(0);
  useEffect(() => {
    if (themeCache) return undefined;
    const fn = () => setTick((n) => n + 1);
    themeListeners.add(fn);
    loadThemeColor();
    return () => {
      themeListeners.delete(fn);
    };
  }, []);
  return themeCache ?? FALLBACK_THEME_COLOR;
}

/** 样式 0（不显示）的占位图形：与 BuyStyleSettings 同形（灰色禁选圈）。 */
function HiddenStyleGlyph() {
  return (
    <svg width="24" height="24" viewBox="0 0 24 24" fill="none" aria-hidden="true">
      <circle cx="12" cy="12" r="9" stroke="#b6bcc4" strokeWidth="2" />
      <line x1="5.8" y1="5.8" x2="18.2" y2="18.2" stroke="#b6bcc4" strokeWidth="2" />
    </svg>
  );
}

function picOrNull(input: unknown): PcBuyBtnPic | null {
  if (!input || typeof input !== 'object') return null;
  const raw = input as Record<string, unknown>;
  const picUrl = String(raw.picUrl ?? '').trim();
  const picThumb = String(raw.picThumb ?? '').trim();
  if (!picUrl && !picThumb) return null;
  return { picUrl, picThumb };
}

export interface BuyBtnStylePickerProps {
  value: unknown;
  options: BuyBtnStyleOption[];
  onChange: (value: number) => void;
  visualSet?: 'unified' | 'h5BtnType';
  /** visualSet='unified' 且值为 10 时的模块自带图标；'h5BtnType' 且值为 5 时的模块 iconImage。 */
  customIcon?: unknown;
  /** 图标颜色覆盖（缺省 = 主题色，同设置页默认口径）。 */
  iconColor?: string;
}

export default function BuyBtnStylePicker({
  value,
  options,
  onChange,
  visualSet = 'unified',
  customIcon,
  iconColor,
}: BuyBtnStylePickerProps) {
  const themeColor = useBuyStyleThemeColor();
  // 仅统一枚举集需要解析 -1（跟随全局）；钩子无条件调用，内部走会话缓存
  const globalStyle = usePcGlobalBuyStyle();
  const effectiveColor = iconColor?.trim() || themeColor;
  const selectedBg = /^#[0-9a-f]{6}$/i.test(themeColor) ? `${themeColor}14` : 'rgba(101, 196, 170, 0.08)';
  const shellVars = { '--pc-primary': themeColor } as CSSProperties;
  const moduleIcon = picOrNull(customIcon);

  const renderVisual = (optionValue: number) => {
    if (visualSet === 'h5BtnType') {
      return <BuyBtnTypeIcon btnType={optionValue} color={effectiveColor} customIcon={customIcon} size={24} />;
    }
    if (optionValue === 0) return <HiddenStyleGlyph />;
    if (optionValue === -1) {
      const resolved = resolvePcBuyBtn(-1, { global: globalStyle });
      if (!resolved.show) return <HiddenStyleGlyph />;
      return <PcBuyButtonVisual style={resolved.style} customIcon={resolved.customIcon ?? moduleIcon} />;
    }
    return <PcBuyButtonVisual style={optionValue} customIcon={optionValue === 10 ? moduleIcon : null} />;
  };

  // 隐藏样式过滤（仅统一集；-1 跟随全局不是样式、当前值保留显示，见文件头注）
  const hiddenStyles = visualSet === 'unified' ? (globalStyle?.hiddenStyles ?? []) : [];
  const isCurrentValue = (optionValue: number) =>
    value !== undefined && value !== null && String(value) === String(optionValue);
  const visibleOptions = options.filter(
    (o) => o.value === -1 || !hiddenStyles.includes(o.value) || isCurrentValue(o.value),
  );

  return (
    <div style={{ display: 'flex', flexWrap: 'wrap', gap: 8 }}>
      {visibleOptions.map((o) => {
        const selected = isCurrentValue(o.value);
        const keptHidden = hiddenStyles.includes(o.value);
        return (
          <button
            key={o.value}
            type="button"
            title={o.label}
            aria-pressed={selected}
            onClick={() => onChange(o.value)}
            style={{
              flex: '0 0 auto',
              width: 88,
              padding: '10px 4px 8px',
              display: 'flex',
              flexDirection: 'column',
              alignItems: 'center',
              gap: 8,
              background: selected ? selectedBg : '#fff',
              border: `1px solid ${selected ? themeColor : '#e3e5e8'}`,
              boxShadow: selected ? `0 0 0 1px ${themeColor}` : undefined,
              borderRadius: 8,
              cursor: 'pointer',
              font: 'inherit',
            }}
          >
            <span
              style={{
                ...shellVars,
                height: 34,
                display: 'inline-flex',
                alignItems: 'center',
                justifyContent: 'center',
                color: effectiveColor,
              }}
            >
              {renderVisual(o.value)}
            </span>
            <span
              style={{
                fontSize: 12,
                lineHeight: 1.35,
                color: selected ? '#1f2329' : '#5f6570',
                fontWeight: selected ? 600 : 400,
                textAlign: 'center',
              }}
            >
              {keptHidden ? `${o.short}·已隐藏` : o.short}
            </span>
          </button>
        );
      })}
    </div>
  );
}
