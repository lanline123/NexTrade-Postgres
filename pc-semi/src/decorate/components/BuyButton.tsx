'use client';

/**
 * 全站共享购买按钮（PC 装修商品类模块统一调用）。
 *
 * 统一枚举与全局配置见 `@/lib/buy-style`。本文件负责：
 * 1. `resolveBuyBtnStyle` —— 读时归一化：把各模块的历史数据格式收敛到
 *    统一枚举（**不回写数据**，保存时由编辑器自然写新格式）：
 *    - 数字/数字字符串：统一枚举原样（-1 = 跟随全局，解析到全局配置）；
 *    - PC 商品分组对象型 `{showBtn, btnType}`：btnType 1→3（加号描边）、
 *      2→1（购物车描边）、3→9（圆圈购物车）；showBtn 控制显隐；
 *    - H5 对象型 `{btnType, iconImage, …}`：btnType 1→1、2→1（购物篮近似
 *      购物车）、3→4、4→4（圆减号无对应，近似加号实心）、5→10（自定义图片）；
 *    - 缺省/null/'global'：跟随全局。
 * 2. `BuyButtonVisual` —— 按钮内部视觉（图标/文案/自定义图片），与外层
 *    `.dec-buy-btn dec-buy-btn--N` 类名（modules-source.scss）配套；各模块
 *    保留自己的 <button>/<Link> 包装与加购行为，只换视觉与样式来源。
 *
 * 自定义图片（样式 10）的渲染决定：用 <img> 原样显示。理由：用户上传的
 * 自定义图标多为多色 artwork，CSS mask 只能单色化，会破坏原图；单色
 * 跟随主题色的需求由内置图标（mask 方案）覆盖。
 */

import { buyStyleConfig, BUY_STYLE_FOLLOW_GLOBAL, type BuyStylePic } from '@/lib/buy-style';

export interface ResolvedBuyBtn {
  /** 是否显示（style=0 或对象型 showBtn 关时为 false）。 */
  show: boolean;
  /** 解析后的统一枚举（0–10）。 */
  style: number;
  /** 样式 10 时的生效图标（模块自带优先，全局兜底）；其它样式为 null。 */
  customIcon: BuyStylePic | null;
  /** 图标色覆盖（全局配置）；空串 = 跟随模块/主题色。 */
  iconColor: string;
}

/** PC 商品分组旧对象型 btnType → 统一枚举。 */
const PC_GROUP_BTN_TYPE_MAP: Record<number, number> = { 1: 3, 2: 1, 3: 9 };
/** H5 旧对象型 btnType → 统一枚举（2 购物篮、4 圆减号无精确对应，取近似）。 */
const H5_BTN_TYPE_MAP: Record<number, number> = { 0: 0, 1: 1, 2: 1, 3: 4, 4: 4, 5: 10 };
/** 搜索结果旧 buyStyle（1/2/3）→ 统一枚举（编辑器改版后写 buyStyleV=2，此表仅供存量）。 */
const SEARCH_LEGACY_MAP: Record<number, number> = { 1: 2, 2: 5, 3: 6 };

export function mapSearchLegacyBuyStyle(n: number): number {
  return SEARCH_LEGACY_MAP[n] ?? 2;
}

function picFrom(input: unknown): BuyStylePic | null {
  if (!input || typeof input !== 'object') return null;
  const raw = input as Record<string, unknown>;
  const picUrl = String(raw.picUrl ?? '').trim();
  const picThumb = String(raw.picThumb ?? '').trim();
  if (!picUrl && !picThumb) return null;
  return { picUrl, picThumb };
}

export interface ResolveBuyBtnOptions {
  /**
   * 数值 0 的替代样式（拼团/秒杀存量数据里 0 表示「未配置」而非隐藏时用）：
   * 调用方在数据未经新版编辑器显式保存（无 buyBtnSet 标记）时传默认样式。
   */
  zeroAs?: number;
  /** 模块自带自定义图标（样式 10 时优先于全局 customIcon）。 */
  moduleIcon?: unknown;
}

/** 把任意历史格式的购买按钮配置解析为统一枚举结果（读时归一化）。 */
export function resolveBuyBtnStyle(raw: unknown, opts?: ResolveBuyBtnOptions): ResolvedBuyBtn {
  const global = buyStyleConfig();
  const moduleIcon = picFrom(opts?.moduleIcon);
  const finish = (style: number, show: boolean, iconColor = ''): ResolvedBuyBtn => ({
    show: show && style > 0,
    style,
    customIcon: style === 10 ? moduleIcon ?? (global.customIcon.picUrl || global.customIcon.picThumb ? global.customIcon : null) : null,
    iconColor,
  });
  const fromGlobal = (): ResolvedBuyBtn => finish(global.style, true, global.iconColor);

  if (raw === undefined || raw === null || raw === '' || raw === 'global') return fromGlobal();

  if (typeof raw === 'object') {
    const obj = raw as Record<string, unknown>;
    if ('showBtn' in obj) {
      // PC 商品分组对象型 {showBtn, btnType, …}
      const style = PC_GROUP_BTN_TYPE_MAP[Number(obj.btnType)] ?? 1;
      return finish(style, Boolean(obj.showBtn));
    }
    if ('btnType' in obj) {
      // H5 对象型 {btnType, iconImage, iconColor, …}
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
  if (n === BUY_STYLE_FOLLOW_GLOBAL) return fromGlobal();
  if (n === 0 && opts?.zeroAs !== undefined) return finish(opts.zeroAs, true);
  const style = Number.isInteger(n) && n >= 0 && n <= 10 ? n : 1;
  return finish(style, true);
}

/** 按钮外层类名（与 modules-source.scss 的 .dec-buy-btn--N 配套）。 */
export function buyBtnClassName(style: number, extra?: string): string {
  return `dec-buy-btn dec-buy-btn--${style}${extra ? ` ${extra}` : ''}`;
}

function maskIcon(kind: 'cart' | 'cart-circle' | 'plus', outline: boolean) {
  return (
    <i className="dec-buy-btn__icon">
      <span
        className={`dec-buy-btn__ico dec-buy-btn__ico--${kind}${outline ? ' is-outline' : ''}`}
        aria-hidden="true"
      />
    </i>
  );
}

/** 购买按钮内部视觉：图标款（mask 图标）/ 文案款 / 自定义图片（<img> 原样）。 */
export function BuyButtonVisual({
  style,
  customIcon,
}: {
  style: number;
  customIcon?: BuyStylePic | null;
}) {
  if (style === 10) {
    const src = customIcon?.picUrl || customIcon?.picThumb || '';
    if (src) {
      // 自定义图片原样显示（多色 artwork 不做 mask 单色化）
      return <img className="dec-buy-btn__img" src={src} alt="" aria-hidden="true" />;
    }
    // 未配置图标时回落购物车描边，避免出现空按钮
    return maskIcon('cart', true);
  }
  if (style === 9) return maskIcon('cart-circle', false);
  if (style >= 1 && style <= 4) {
    return maskIcon(style <= 2 ? 'cart' : 'plus', style === 1 || style === 3);
  }
  if (style >= 5) {
    return <span className="dec-buy-btn__text">{style >= 7 ? '马上抢' : '购买'}</span>;
  }
  return null;
}
