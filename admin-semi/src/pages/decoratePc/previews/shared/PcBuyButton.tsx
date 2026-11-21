import { useEffect, useState } from 'react';
import { apiGet } from '../../../../api';
import { DEFAULT_OBJ_STYLE, sanitizeObjStyle, type ObjBuyStyle } from '../../../../components/buyStyleObj';
import './pcBuyButton.css';

/**
 * PC 装修编辑器共享购买按钮（预览侧，与 pc-semi 前台 BuyButton 同口径）。
 *
 * 统一枚举（全站一致，见 pc-semi/src/lib/buy-style.ts）：
 *   -1 = 跟随全局（解析到后台「购买按钮样式」）  0 = 不显示
 *    1 = 购物车（描边）  2 = 购物车（实心）  3 = 加号（描边）  4 = 加号（实心）
 *    5 = "购买"（描边）  6 = "购买"（实心）  7 = "马上抢"（描边）  8 = "马上抢"（实心）
 *    9 = 圆圈购物车      10 = 自定义图片（模块 buyBtnIcon 优先，全局 customIcon 兜底）
 *   11 = 购物袋（描边） 12 = 购物袋（实心） 13 = 圆圈购物袋  14 = 圆圈加号
 *   15 = 加购购物车      16 = 闪电          17 = "立即购买"（描边） 18 = "立即购买"（实心）
 *   （11–16 图标源自 IconPark 电商类目，2026-11-21 扩充）
 *
 * 全局配置的 hiddenStyles（设置页隐藏的样式）只供选择器过滤选项，本文件
 * 仅透传不消费：解析/渲染对已保存的样式 id 一律照常（隐藏不影响存量渲染）。
 * objStyle（对象型字段组）同理只透传：PC 渲染不消费它，但 H5 编辑器的
 * 跟随解析经本文件的全局缓存读取，sanitizeGlobal 必须原样带出（经
 * components/buyStyleObj 的 sanitizeObjStyle 清洗），洗掉即断 H5 链路。
 *
 * resolvePcBuyBtn 做读时归一化（不回写数据）：数字/字符串原样；PC 商品分组
 * 旧对象型 {showBtn, btnType} 按 1→3、2→1、3→9 映射；H5 对象型同前台映射表。
 * 编辑器载入时另有 normalizeLegacyBuyBtn 把存量数据在工作副本里转成新格式，
 * 本函数是预览层的兜底（草稿/外部数据未经编辑器归一化时仍正确渲染）。
 */

export interface PcBuyBtnPic {
  picUrl: string;
  picThumb: string;
}

export interface PcResolvedBuyBtn {
  show: boolean;
  style: number;
  customIcon: PcBuyBtnPic | null;
  iconColor: string;
}

export interface GlobalBuyStyle {
  style: number;
  customIcon: PcBuyBtnPic;
  iconColor: string;
  /** 设置页隐藏的样式 id（仅选择器过滤用，渲染不消费）；可选——存量调用方
   * 手工构造的全局对象可不带此字段，消费方按空列表处理。 */
  hiddenStyles?: number[];
  /** 对象型字段组（H5 对象型模块跟随全局时消费）；PC 侧只透传不消费——
   * 本文件的 sanitize 若把它洗掉，H5 编辑器经此缓存就读不到全局原值。
   * 可选——存量调用方手工构造的全局对象可不带，消费方走默认组。 */
  objStyle?: ObjBuyStyle;
}

const DEFAULT_GLOBAL: GlobalBuyStyle = {
  style: 1,
  customIcon: { picUrl: '', picThumb: '' },
  iconColor: '',
  hiddenStyles: [],
  objStyle: DEFAULT_OBJ_STYLE,
};

/* ---------- 全局配置懒加载（解析 -1 时用；整个编辑器会话只拉一次） ---------- */

let globalCache: GlobalBuyStyle | null = null;
let globalPromise: Promise<void> | null = null;
const globalListeners = new Set<() => void>();

function sanitizeGlobal(input: unknown): GlobalBuyStyle {
  if (!input || typeof input !== 'object') return DEFAULT_GLOBAL;
  const raw = input as Record<string, any>;
  const styleNum = Number(raw.style);
  const icon = raw.customIcon && typeof raw.customIcon === 'object' ? raw.customIcon : {};
  const hidden = Array.isArray(raw.hiddenStyles)
    ? [...new Set(raw.hiddenStyles.map(Number).filter((n) => Number.isInteger(n) && n >= 0 && n <= 18))]
    : [];
  return {
    style: Number.isInteger(styleNum) && styleNum >= 0 && styleNum <= 18 ? styleNum : DEFAULT_GLOBAL.style,
    customIcon: {
      picUrl: String(icon.picUrl ?? '').trim(),
      picThumb: String(icon.picThumb ?? '').trim(),
    },
    iconColor: String(raw.iconColor ?? '').trim(),
    hiddenStyles: hidden,
    objStyle: sanitizeObjStyle(raw.objStyle),
  };
}

function loadGlobalBuyStyle(): void {
  if (globalCache || globalPromise) return;
  globalPromise = apiGet<Record<string, any>>('/setting/config/buyStyleSettings')
    .then((data) => {
      globalCache = sanitizeGlobal(data);
    })
    .catch(() => {
      globalCache = DEFAULT_GLOBAL;
    })
    .finally(() => {
      globalListeners.forEach((fn) => fn());
      globalListeners.clear();
    });
}

/** 订阅全局购买样式（未加载完成时返回 null，组件先按内置默认渲染，加载后自动重渲）。 */
export function usePcGlobalBuyStyle(): GlobalBuyStyle | null {
  const [, setTick] = useState(0);
  useEffect(() => {
    if (globalCache) return undefined;
    const fn = () => setTick((n) => n + 1);
    globalListeners.add(fn);
    loadGlobalBuyStyle();
    return () => {
      globalListeners.delete(fn);
    };
  }, []);
  return globalCache;
}

/* ---------- 读时归一化（与 pc-semi BuyButton.resolveBuyBtnStyle 同表） ---------- */

const PC_GROUP_BTN_TYPE_MAP: Record<number, number> = { 1: 3, 2: 1, 3: 9 };
const H5_BTN_TYPE_MAP: Record<number, number> = { 0: 0, 1: 1, 2: 1, 3: 4, 4: 4, 5: 10 };
/** 搜索结果旧 buyStyle（1/2/3）→ 统一枚举（仅 buyStyleV<2 的存量数据用）。 */
export const SEARCH_LEGACY_MAP: Record<number, number> = { 1: 2, 2: 5, 3: 6 };

function picFrom(input: unknown): PcBuyBtnPic | null {
  if (!input || typeof input !== 'object') return null;
  const raw = input as Record<string, unknown>;
  const picUrl = String(raw.picUrl ?? '').trim();
  const picThumb = String(raw.picThumb ?? '').trim();
  if (!picUrl && !picThumb) return null;
  return { picUrl, picThumb };
}

export interface ResolvePcBuyBtnOptions {
  /** 数值 0 的替代样式（拼团/秒杀存量 0=未配置时由调用方传 4；编辑器归一化后一般用不到）。 */
  zeroAs?: number;
  /** 模块自带自定义图标（样式 10 时优先于全局 customIcon）。 */
  moduleIcon?: unknown;
  /** 全局配置（usePcGlobalBuyStyle 的结果；未加载时用内置默认）。 */
  global?: GlobalBuyStyle | null;
}

export function resolvePcBuyBtn(raw: unknown, opts?: ResolvePcBuyBtnOptions): PcResolvedBuyBtn {
  const global = opts?.global ?? DEFAULT_GLOBAL;
  const moduleIcon = picFrom(opts?.moduleIcon);
  const finish = (style: number, show: boolean, iconColor = ''): PcResolvedBuyBtn => ({
    show: show && style > 0,
    style,
    customIcon: style === 10 ? moduleIcon ?? (global.customIcon.picUrl || global.customIcon.picThumb ? global.customIcon : null) : null,
    iconColor,
  });
  const fromGlobal = (): PcResolvedBuyBtn => finish(global.style, true, global.iconColor);

  if (raw === undefined || raw === null || raw === '' || raw === 'global') return fromGlobal();

  if (typeof raw === 'object') {
    const obj = raw as Record<string, unknown>;
    if ('showBtn' in obj) {
      const style = PC_GROUP_BTN_TYPE_MAP[Number(obj.btnType)] ?? 1;
      return finish(style, Boolean(obj.showBtn));
    }
    if ('btnType' in obj) {
      const style = H5_BTN_TYPE_MAP[Number(obj.btnType)] ?? 1;
      const icon = picFrom(obj.iconImage ?? obj.customIcon);
      return {
        show: style > 0,
        style,
        customIcon: style === 10 ? icon ?? moduleIcon ?? (global.customIcon.picUrl ? global.customIcon : null) : null,
        iconColor: String(obj.iconColor ?? ''),
      };
    }
    return fromGlobal();
  }

  const n = Number(raw);
  if (!Number.isFinite(n)) return fromGlobal();
  if (n === -1) return fromGlobal();
  if (n === 0 && opts?.zeroAs !== undefined) return finish(opts.zeroAs, true);
  const style = Number.isInteger(n) && n >= 0 && n <= 18 ? n : 1;
  return finish(style, true);
}

/* ---------- 视觉（与前台 BuyButtonVisual 同构：mask 图标 / 文案 / 自定义图片） ---------- */

function maskIcon(
  kind: 'cart' | 'cart-circle' | 'plus' | 'bag' | 'bag-circle' | 'plus-circle' | 'cart-add' | 'lightning',
  outline: boolean,
) {
  return (
    <i className="pc-buy-btn__icon">
      <span className={`pc-buy-btn__ico pc-buy-btn__ico--${kind}${outline ? ' is-outline' : ''}`} aria-hidden="true" />
    </i>
  );
}

/** 购买按钮内部视觉：宿主（各模块自己的按钮壳）负责尺寸/底色，本组件只出内容。 */
export function PcBuyButtonVisual({
  style,
  customIcon,
}: {
  style: number;
  customIcon?: PcBuyBtnPic | null;
}) {
  if (style === 10) {
    const src = customIcon?.picUrl || customIcon?.picThumb || '';
    if (src) {
      return <img className="pc-buy-btn__img" src={src} alt="" aria-hidden="true" />;
    }
    return maskIcon('cart', true);
  }
  if (style === 9) return maskIcon('cart-circle', false);
  if (style >= 1 && style <= 4) {
    return maskIcon(style <= 2 ? 'cart' : 'plus', style === 1 || style === 3);
  }
  if (style === 11 || style === 12) return maskIcon('bag', style === 11);
  if (style === 13) return maskIcon('bag-circle', false);
  if (style === 14) return maskIcon('plus-circle', false);
  if (style === 15) return maskIcon('cart-add', false);
  if (style === 16) return maskIcon('lightning', false);
  if ((style >= 5 && style <= 8) || style === 17 || style === 18) {
    const solid = style === 6 || style === 8 || style === 18;
    const text = style === 17 || style === 18 ? '立即购买' : style >= 7 ? '马上抢' : '购买';
    return <span className={`pc-buy-btn__text pc-buy-btn__text--${solid ? 'solid' : 'outline'}`}>{text}</span>;
  }
  return null;
}
