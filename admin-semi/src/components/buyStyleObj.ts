/**
 * 全局「购买按钮样式」对象型字段组（objStyle）+ 两族跟随解析（纯逻辑，无 React）。
 *
 * 背景（2026-11-21，我方新增；评估见 buybtn-global-fields-eval 报告）：
 * 全局 buyStyle 配置在统一枚举（style/iconColor/customIcon/hiddenStyles）之外
 * 新增嵌套子对象 `objStyle`，收纳 H5 对象型一族（分组滑动/商品分组/商品轮播/
 * 商品A1，共用 CommonBuyButton）模块级的 13 个外观字段，字段名与模块侧逐字
 * 一致，跟随时浅合并零改名。`showBtn/wrap/wrapAlign` 是模块语境字段，永不
 * 进全局、跟随时不被覆盖。
 *
 * 两条铁律（与后端 buy_style.rs 同口径，严禁「优化」掉）：
 * ① objStyle 的渐变色默认必须是 ""（= 跟随主题色，与现行渲染的黑圈哨兵
 *    口径一致），严禁填模块数据出厂值 #333333；
 * ② objStyle.iconColor 与顶层 iconColor 同名但语义相反（顶层空串 = 跟随
 *    主题；对象型恒为显式色），分开存：本组清洗时空回落 #ffffff。
 *
 * 本文件与 uniapp-react/src/utils/buyStyle.ts 逐行同构（默认值/清洗/解析/
 * 降级表），两端互注、改一处必须同步另一处。
 */

export interface BuyStylePic {
  picUrl: string;
  picThumb: string;
}

/** 对象型字段组（13 字段，与 H5 对象型模块 buyBtnStyle 对象同名同型）。 */
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

/** objStyle 内置默认（= 现行渲染行为；与后端 default_obj_style 逐字一致）。 */
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

/** 全局配置中与跟随解析相关的最小形状（各消费端自己的全局类型与之结构兼容）。 */
export interface BuyStyleGlobalLike {
  style: number;
  iconColor: string;
  objStyle?: unknown;
}

function cleanText(v: unknown, max: number): string {
  return String(v ?? '').trim().slice(0, max);
}

function cleanPic(input: unknown): BuyStylePic {
  if (!input || typeof input !== 'object') return { picUrl: '', picThumb: '' };
  const raw = input as Record<string, unknown>;
  return { picUrl: cleanText(raw.picUrl, 1024), picThumb: cleanText(raw.picThumb, 1024) };
}

function clampNum(v: unknown, min: number, max: number, dflt: number): number {
  const n = Number(v);
  if (!Number.isFinite(n)) return dflt;
  return Math.min(max, Math.max(min, Math.round(n)));
}

function requiredColor(v: unknown, dflt: string): string {
  const s = cleanText(v, 32);
  return s === '' ? dflt : s;
}

const GRADIENT_TYPES = ['upDown', 'leftRight', 'diagonal', 'purity'];

/** 清洗一份 objStyle（与后端 sanitize_obj_style 同口径）；非法输入返回默认组。 */
export function sanitizeObjStyle(input: unknown): ObjBuyStyle {
  if (!input || typeof input !== 'object') return { ...DEFAULT_OBJ_STYLE, iconImage: { ...DEFAULT_OBJ_STYLE.iconImage } };
  const raw = input as Record<string, unknown>;
  const btn = Number(raw.btnType);
  return {
    btnType: Number.isInteger(btn) && btn >= 1 && btn <= 5 ? btn : 1,
    iconImage: cleanPic(raw.iconImage),
    iconSize: clampNum(raw.iconSize, 12, 40, 14),
    text: cleanText(raw.text, 32),
    textColor: requiredColor(raw.textColor, '#ffffff'),
    textSize: clampNum(raw.textSize, 10, 24, 12),
    iconColor: requiredColor(raw.iconColor, '#ffffff'),
    gradientType: GRADIENT_TYPES.includes(String(raw.gradientType)) ? String(raw.gradientType) : 'leftRight',
    gradientColorA: cleanText(raw.gradientColorA, 32),
    gradientColorB: cleanText(raw.gradientColorB, 32),
    radius: clampNum(raw.radius, 0, 20, 16),
    paddingTopBottom: clampNum(raw.paddingTopBottom, 0, 10, 0),
    paddingLeftRight: clampNum(raw.paddingLeftRight, 0, 10, 10),
  };
}

/**
 * 对象型跟随解析（读时解析、不回写数据）：模块对象开启 followGlobal 时，
 * 13 个外观字段全量取全局 objStyle（缺失/老配置走默认组 = 出厂行为），
 * showBtn/wrap/wrapAlign 等模块语境字段与 followGlobal 本身保留模块值。
 * 未开启跟随时原样返回入参（同一引用，渲染零变化）。
 */
export function resolveObjBuyButton<T extends Record<string, any>>(moduleObj: T, global: BuyStyleGlobalLike | null | undefined): T {
  if (!moduleObj || typeof moduleObj !== 'object') return moduleObj;
  if (Number(moduleObj.followGlobal) !== 1) return moduleObj;
  const obj = sanitizeObjStyle(global?.objStyle);
  return { ...moduleObj, ...obj, followGlobal: 1 };
}

/**
 * 全局统一枚举（0–18）→ H5 数字型（0–8）降级映射（评估 §3.2 定表）。
 * 0–8 恒等；9–18 近似：10 自定义图片在 H5 数字型无对应形态 → 1（有损点），
 * 16 闪电 → 8「马上抢」实心（有损点，闪购语义最近）。
 * 注意：与 PC 侧既有映射表（PC_GROUP_BTN_TYPE_MAP / H5_BTN_TYPE_MAP）方向
 * 相反、分开维护，改一处不要顺手改另一处。
 */
export const GLOBAL_TO_H5_DIGITAL: Record<number, number> = {
  0: 0, 1: 1, 2: 2, 3: 3, 4: 4, 5: 5, 6: 6, 7: 7, 8: 8,
  9: 2, 10: 1, 11: 1, 12: 2, 13: 2, 14: 4, 15: 1, 16: 8, 17: 5, 18: 6,
};

export interface ResolvedDigitalBuyBtn {
  /** 解析后的 H5 数字型样式（0–8）；入参为 undefined/null 时原样透传（调用方自有缺省语义）。 */
  style: number | undefined;
  /** 跟随全局且全局 iconColor 非空时的图标色覆盖；不跟随时为空串（不覆盖）。 */
  iconColor: string;
}

/**
 * 数字型跟随解析：raw === -1 时取全局 style 降级到 0–8，并带出全局
 * iconColor（宿主以 --general 局部变量作用于按钮元素本身）；其余值原样
 * 透传（存量 0–8 逐字不变，越界/缺省由调用方既有口径处理）。
 */
export function resolveH5DigitalBuyBtn(raw: unknown, global: BuyStyleGlobalLike | null | undefined): ResolvedDigitalBuyBtn {
  if (raw === undefined || raw === null) return { style: undefined, iconColor: '' };
  const n = Number(raw);
  if (n === -1) {
    const gStyle = Number.isInteger(Number(global?.style)) ? Number(global?.style) : 1;
    return { style: GLOBAL_TO_H5_DIGITAL[gStyle] ?? 1, iconColor: String(global?.iconColor ?? '').trim() };
  }
  return { style: Number.isFinite(n) ? n : undefined, iconColor: '' };
}
