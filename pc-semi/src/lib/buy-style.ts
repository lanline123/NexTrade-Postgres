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

/**
 * 对象型字段组（2026-11-21 字段补齐，我方新增）：H5 对象型一族（分组滑动/
 * 商品分组/商品轮播/商品A1）模块级外观字段的全局超集，字段名与模块侧逐字
 * 一致。PC 不消费本组（PC 模块外观全由样式号 CSS 决定），此处仅做类型与
 * 清洗透传——bootstrap 链路若把它洗掉，配置往返即丢字段。
 * 两条铁律（与后端 buy_style.rs 同口径）：渐变色默认 ""（跟随主题色），
 * 严禁 #333333；本组 iconColor 与顶层 iconColor 语义相反（恒显式色），
 * 分开存、空回落 #ffffff，严禁合并。
 */
export interface ObjBuyStyle {
  btnType: number;
  iconImage: BuyStylePic;
  iconSize: number;
  text: string;
  textColor: string;
  textSize: number;
  iconColor: string;
  gradientType: string;
  gradientColorA: string;
  gradientColorB: string;
  radius: number;
  paddingTopBottom: number;
  paddingLeftRight: number;
}

export interface BuyStyleConfig {
  /** 全局默认样式（统一枚举 0–18）。 */
  style: number;
  /** 样式 10 时的全局自定义图标（模块未自带图标时兜底）。 */
  customIcon: BuyStylePic;
  /** 全局图标色覆盖；空串 = 跟随模块/主题色。 */
  iconColor: string;
  /** 对象型字段组（仅透传，PC 不消费）。 */
  objStyle: ObjBuyStyle;
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

/** objStyle 内置默认（与后端 default_obj_style 逐字一致）。 */
export const DEFAULT_OBJ_STYLE: ObjBuyStyle = {
  btnType: 1,
  iconImage: { picUrl: '', picThumb: '' },
  iconSize: 14,
  text: '',
  textColor: '#ffffff',
  textSize: 12,
  iconColor: '#ffffff',
  gradientType: 'leftRight',
  gradientColorA: '',
  gradientColorB: '',
  radius: 16,
  paddingTopBottom: 0,
  paddingLeftRight: 10,
};

/** 内置默认（与后端 default_buy_style、admin 设置页默认一致）。 */
export const DEFAULT_BUY_STYLE: BuyStyleConfig = {
  style: 1,
  customIcon: { picUrl: '', picThumb: '' },
  iconColor: '',
  objStyle: DEFAULT_OBJ_STYLE,
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

function clampNum(v: unknown, min: number, max: number, dflt: number): number {
  const n = Number(v);
  if (!Number.isFinite(n)) return dflt;
  return Math.min(max, Math.max(min, Math.round(n)));
}

const OBJ_GRADIENT_TYPES = ['upDown', 'leftRight', 'diagonal', 'purity'];

/** objStyle 透传清洗（与后端 sanitize_obj_style 同口径；PC 只透传不消费）。 */
function cleanObjStyle(input: unknown): ObjBuyStyle {
  if (!input || typeof input !== 'object') return DEFAULT_OBJ_STYLE;
  const raw = input as Record<string, unknown>;
  const btn = Number(raw.btnType);
  const requiredColor = (v: unknown) => {
    const s = String(v ?? '').trim().slice(0, 32);
    return s === '' ? '#ffffff' : s;
  };
  return {
    btnType: Number.isInteger(btn) && btn >= 1 && btn <= 5 ? btn : 1,
    iconImage: cleanPic(raw.iconImage),
    iconSize: clampNum(raw.iconSize, 12, 40, 14),
    text: String(raw.text ?? '').trim().slice(0, 32),
    textColor: requiredColor(raw.textColor),
    textSize: clampNum(raw.textSize, 10, 24, 12),
    iconColor: requiredColor(raw.iconColor),
    gradientType: OBJ_GRADIENT_TYPES.includes(String(raw.gradientType)) ? String(raw.gradientType) : 'leftRight',
    gradientColorA: String(raw.gradientColorA ?? '').trim().slice(0, 32),
    gradientColorB: String(raw.gradientColorB ?? '').trim().slice(0, 32),
    radius: clampNum(raw.radius, 0, 20, 16),
    paddingTopBottom: clampNum(raw.paddingTopBottom, 0, 10, 0),
    paddingLeftRight: clampNum(raw.paddingLeftRight, 0, 10, 10),
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
    objStyle: cleanObjStyle(raw.objStyle),
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
