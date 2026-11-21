/**
 * 全站购买按钮样式（后台「购买按钮样式」buyStyle 配置 + 统一枚举契约）。
 *
 * 统一枚举（PC 装修所有商品类模块共用，编辑器同口径）：
 *   -1 = 跟随全局（模块值缺省/未配置时亦同）
 *    0 = 不显示
 *    1 = 购物车（描边）  2 = 购物车（实心）  3 = 加号（描边）  4 = 加号（实心）
 *    5 = "购买"（描边）  6 = "购买"（实心）  7 = "马上抢"（描边）  8 = "马上抢"（实心）
 *    9 = 圆圈购物车      10 = 自定义图片（图标取模块自带或全局 customIcon）
 *   11 = 购物袋（描边） 12 = 购物袋（实心） 13 = 圆圈购物袋  14 = 圆圈加号
 *   15 = 加购购物车      16 = 闪电          17 = "立即购买"（描边） 18 = "立即购买"（实心）
 *
 * 1–8 沿用旧版 ProductButton.vue 语义；9 收编旧商品分组对象型 btnType=3；
 * 10 收编 H5 对象型 btnType=5（自定义图片）；11–18 为 2026-11-21 扩充
 * （11–16 图标源自 IconPark 电商类目，Apache-2.0）。
 *
 * 配置里的 hiddenStyles（后台隐藏的样式）前台不消费：渲染只认样式 id，
 * 被隐藏的样式若已被页面/全局选用，照常渲染（隐藏仅约束编辑器选择器）。
 *
 * 配置由 layout 服务端预取（common/config/buyStyle）→ 内联 bootstrap →
 * setBuyStyleConfig 注入，与 priceDisplay 同一条链路（见 lib/currency.ts）。
 */

export interface BuyStylePic {
  picUrl: string;
  picThumb: string;
}

export interface BuyStyleConfig {
  /** 全局默认样式（统一枚举 0–18）。 */
  style: number;
  /** 样式 10 时的全局自定义图标（模块未自带图标时兜底）。 */
  customIcon: BuyStylePic;
  /** 全局图标色覆盖；空串 = 跟随模块/主题色。 */
  iconColor: string;
}

/** 模块值：跟随全局。 */
export const BUY_STYLE_FOLLOW_GLOBAL = -1;

/** 统一枚举选项（编辑器设置面板同表，两端保持一致）。 */
export const BUY_BUTTON_OPTIONS: Array<{ value: number; label: string }> = [
  { value: -1, label: '跟随全局' },
  { value: 0, label: '不显示' },
  { value: 1, label: '购物车（描边）' },
  { value: 2, label: '购物车（实心）' },
  { value: 3, label: '加号（描边）' },
  { value: 4, label: '加号（实心）' },
  { value: 5, label: '购买（描边）' },
  { value: 6, label: '购买（实心）' },
  { value: 7, label: '马上抢（描边）' },
  { value: 8, label: '马上抢（实心）' },
  { value: 9, label: '圆圈购物车' },
  { value: 10, label: '自定义图片' },
  { value: 11, label: '购物袋（描边）' },
  { value: 12, label: '购物袋（实心）' },
  { value: 13, label: '圆圈购物袋' },
  { value: 14, label: '圆圈加号' },
  { value: 15, label: '加购购物车' },
  { value: 16, label: '闪电' },
  { value: 17, label: '立即购买（描边）' },
  { value: 18, label: '立即购买（实心）' },
];

/** 内置默认（与后端 default_buy_style、admin 设置页默认一致）。 */
export const DEFAULT_BUY_STYLE: BuyStyleConfig = {
  style: 1,
  customIcon: { picUrl: '', picThumb: '' },
  iconColor: '',
};

let buyStyleState: BuyStyleConfig = DEFAULT_BUY_STYLE;
let buyStyleHydrated = false;

function cleanPic(input: unknown): BuyStylePic {
  if (!input || typeof input !== 'object') return { picUrl: '', picThumb: '' };
  const raw = input as Record<string, unknown>;
  return {
    picUrl: String(raw.picUrl ?? '').trim().slice(0, 1024),
    picThumb: String(raw.picThumb ?? '').trim().slice(0, 1024),
  };
}

function sanitizeBuyStyle(input: unknown): BuyStyleConfig | null {
  if (!input || typeof input !== 'object') return null;
  const raw = input as Record<string, unknown>;
  const styleNum = Number(raw.style);
  const style =
    Number.isFinite(styleNum) && Number.isInteger(styleNum) && styleNum >= 0 && styleNum <= 18
      ? styleNum
      : DEFAULT_BUY_STYLE.style;
  return {
    style,
    customIcon: cleanPic(raw.customIcon),
    iconColor: String(raw.iconColor ?? '').trim().slice(0, 32),
  };
}

/** layout 服务端 bootstrap 注入后台购买按钮样式配置（SSR 与客户端各调一次，内容一致）。 */
export function setBuyStyleConfig(input: unknown): void {
  const clean = sanitizeBuyStyle(input);
  if (clean) {
    buyStyleState = clean;
    buyStyleHydrated = true;
  }
}

/** 当前生效的购买按钮样式配置（客户端首次使用时从 layout 内联 bootstrap 懒加载）。 */
export function buyStyleConfig(): BuyStyleConfig {
  if (!buyStyleHydrated && typeof document !== 'undefined') {
    buyStyleHydrated = true;
    try {
      const raw = document.getElementById('pc-header-bootstrap')?.textContent;
      const parsed = raw ? (JSON.parse(raw) as any) : null;
      const clean = sanitizeBuyStyle(parsed?.buyStyle);
      if (clean) buyStyleState = clean;
    } catch {
      /* bootstrap 缺失时用内置默认 */
    }
  }
  return buyStyleState;
}
