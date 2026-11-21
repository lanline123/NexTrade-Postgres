/**
 * 全局「购买按钮样式」（buyStyle 配置）H5 消费侧：类型 / 清洗 / 两族跟随解析。
 *
 * 背景（2026-11-21，我方新增；评估见 buybtn-global-fields-eval 报告）：
 * 全局配置除统一枚举（style/iconColor/customIcon）外，新增嵌套子对象
 * `objStyle` 收纳对象型一族（分组滑动/商品分组/商品轮播/商品A1，共用
 * CommonBuyButton）的 13 个外观字段，字段名与模块侧逐字一致。模块开启
 * `followGlobal`（对象型）或 buyBtnStyle = -1（数字型）后，渲染层读时解析
 * 本配置（不回写模块数据）。配置经 store/config 的 loadBaseConfigData 与
 * priceDisplay 同通道拉取（公开端点 common/config/buyStyle），失败静默
 * 保留旧值，未拉到时消费侧回落 DEFAULT_BUY_STYLE（与后端默认一致）。
 *
 * 两条铁律（与后端 buy_style.rs 同口径）：objStyle 渐变默认 ""（跟随主题
 * 色），严禁 #333333；objStyle.iconColor 与顶层 iconColor 语义相反（恒
 * 显式色），分开存、空回落 #ffffff。
 *
 * 本文件与 admin-semi/src/components/buyStyleObj.ts 逐行同构（默认值/清洗/
 * 解析/降级表），两端互注、改一处必须同步另一处。
 */

export interface BuyStylePic {
    picUrl: string;
    picThumb: string;
}

/** 对象型字段组（13 字段，与对象型模块 buyBtnStyle 对象同名同型）。 */
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
    /** 全局默认样式（统一枚举 0–18），数字型 -1 跟随时降级到 0–8 消费。 */
    style: number;
    /** 样式 10 的全局自定义图标（H5 数字型无对应形态，仅透传）。 */
    customIcon: BuyStylePic;
    /** 全局图标色覆盖；空串 = 跟随主题色（数字型跟随时作用于按钮宿主）。 */
    iconColor: string;
    /** 对象型字段组（对象型跟随时消费）。 */
    objStyle: ObjBuyStyle;
}

/** objStyle 内置默认（= 现行渲染行为；与后端 default_obj_style 逐字一致）。 */
export const DEFAULT_OBJ_STYLE: ObjBuyStyle = {
    btnType: 1,
    iconImage: { picUrl: "", picThumb: "" },
    iconSize: 14,
    text: "",
    textColor: "#ffffff",
    textSize: 12,
    iconColor: "#ffffff",
    gradientType: "leftRight",
    gradientColorA: "",
    gradientColorB: "",
    radius: 16,
    paddingTopBottom: 0,
    paddingLeftRight: 10
};

/** 内置默认配置（与后端 default_buy_style 一致）。 */
export const DEFAULT_BUY_STYLE: BuyStyleConfig = {
    style: 1,
    customIcon: { picUrl: "", picThumb: "" },
    iconColor: "",
    objStyle: DEFAULT_OBJ_STYLE
};

function cleanText(v: unknown, max: number): string {
    return String(v ?? "").trim().slice(0, max);
}

function cleanPic(input: unknown): BuyStylePic {
    if (!input || typeof input !== "object") return { picUrl: "", picThumb: "" };
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
    return s === "" ? dflt : s;
}

const GRADIENT_TYPES = ["upDown", "leftRight", "diagonal", "purity"];

/** 清洗一份 objStyle（与后端 sanitize_obj_style 同口径）；非法输入返回默认组。 */
export function sanitizeObjStyle(input: unknown): ObjBuyStyle {
    if (!input || typeof input !== "object") return { ...DEFAULT_OBJ_STYLE, iconImage: { ...DEFAULT_OBJ_STYLE.iconImage } };
    const raw = input as Record<string, unknown>;
    const btn = Number(raw.btnType);
    return {
        btnType: Number.isInteger(btn) && btn >= 1 && btn <= 5 ? btn : 1,
        iconImage: cleanPic(raw.iconImage),
        iconSize: clampNum(raw.iconSize, 12, 40, 14),
        text: cleanText(raw.text, 32),
        textColor: requiredColor(raw.textColor, "#ffffff"),
        textSize: clampNum(raw.textSize, 10, 24, 12),
        iconColor: requiredColor(raw.iconColor, "#ffffff"),
        gradientType: GRADIENT_TYPES.includes(String(raw.gradientType)) ? String(raw.gradientType) : "leftRight",
        gradientColorA: cleanText(raw.gradientColorA, 32),
        gradientColorB: cleanText(raw.gradientColorB, 32),
        radius: clampNum(raw.radius, 0, 20, 16),
        paddingTopBottom: clampNum(raw.paddingTopBottom, 0, 10, 0),
        paddingLeftRight: clampNum(raw.paddingLeftRight, 0, 10, 10)
    };
}

/** 清洗一份下发的 buyStyle 配置（与后端/PC 同口径）；非法输入返回 null。 */
export function sanitizeBuyStyle(input: unknown): BuyStyleConfig | null {
    if (!input || typeof input !== "object") return null;
    const raw = input as Record<string, unknown>;
    const styleNum = Number(raw.style);
    return {
        style: Number.isInteger(styleNum) && styleNum >= 0 && styleNum <= 18 ? styleNum : DEFAULT_BUY_STYLE.style,
        customIcon: cleanPic(raw.customIcon),
        iconColor: cleanText(raw.iconColor, 32),
        objStyle: sanitizeObjStyle(raw.objStyle)
    };
}

/**
 * 对象型跟随解析（读时解析、不回写数据）：模块对象开启 followGlobal 时，
 * 13 个外观字段全量取全局 objStyle，showBtn/wrap/wrapAlign 等模块语境
 * 字段与 followGlobal 本身保留模块值。未开启时原样返回入参（同一引用）。
 * global 为 null（配置未拉到）时用默认组兜底 = 出厂行为。
 */
export function resolveObjBuyButton<T extends Record<string, any>>(moduleObj: T, global: BuyStyleConfig | null | undefined): T {
    if (!moduleObj || typeof moduleObj !== "object") return moduleObj;
    if (Number(moduleObj.followGlobal) !== 1) return moduleObj;
    const obj = sanitizeObjStyle(global?.objStyle);
    return { ...moduleObj, ...obj, followGlobal: 1 };
}

/**
 * 全局统一枚举（0–18）→ H5 数字型（0–8）降级映射（评估 §3.2 定表）。
 * 0–8 恒等；9–18 近似：10 自定义图片在 H5 数字型无对应形态 → 1（有损点），
 * 16 闪电 → 8「马上抢」实心（有损点，闪购语义最近）。
 * 与 PC 侧既有映射表方向相反、分开维护，改一处不要顺手改另一处。
 */
export const GLOBAL_TO_H5_DIGITAL: Record<number, number> = {
    0: 0, 1: 1, 2: 2, 3: 3, 4: 4, 5: 5, 6: 6, 7: 7, 8: 8,
    9: 2, 10: 1, 11: 1, 12: 2, 13: 2, 14: 4, 15: 1, 16: 8, 17: 5, 18: 6
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
export function resolveH5BuyBtnStyle(raw: unknown, global: BuyStyleConfig | null | undefined): ResolvedDigitalBuyBtn {
    if (raw === undefined || raw === null) return { style: undefined, iconColor: "" };
    const n = Number(raw);
    if (n === -1) {
        const g = global ?? DEFAULT_BUY_STYLE;
        return { style: GLOBAL_TO_H5_DIGITAL[g.style] ?? 1, iconColor: String(g.iconColor ?? "").trim() };
    }
    return { style: Number.isFinite(n) ? n : undefined, iconColor: "" };
}

/** 扁平色族（C 族）按钮三色的解析结果（CSS 可直接用的最终值）。 */
export interface FlatBtnColors {
    gradientStart: string;
    gradientEnd: string;
    textColor: string;
}

/**
 * 扁平色族跟随解析（三期；C 族 = 商品滑动/限时特惠/新人专享，按钮
 * 色以模块根扁平字段 buyBtnGradientStart/End + buyBtnTextColor 存）：
 * 模块开启 buyBtnFollowGlobal=1 时，渐变起止与文字色取全局
 * objStyle 的 gradientColorA/B 与 textColor（评估 §5.3/§5.8 定：
 * 起→A、止→B 无损对应；buyBtnTextColor 与 objStyle.textColor
 * 同义）。全局渐变为空串 = 跟随主题，渲染为 var(--general)
 * （铁律：渐变默认 "" 跟随主题，严禁 #333）。渐变方向不映射
 * （C 族方向由各模块 CSS 固定）；数字型 buyBtnStyle 不在映射
 * 内、恒归模块自管。未开启跟随时模块值原样（空/缺失回落模块
 * 出厂默认 dflt，与各模块现行 || 口径一致）。读时解析、从不
 * 回写模块数据；切回自定义即恢复模块原值。
 */
export function resolveFlatBtnColors(
    data: Record<string, any> | null | undefined,
    global: BuyStyleConfig | null | undefined,
    dflt: FlatBtnColors
): FlatBtnColors {
    if (Number(data?.buyBtnFollowGlobal) === 1) {
        const obj = sanitizeObjStyle(global?.objStyle);
        return {
            gradientStart: obj.gradientColorA === "" ? "var(--general)" : obj.gradientColorA,
            gradientEnd: obj.gradientColorB === "" ? "var(--general)" : obj.gradientColorB,
            textColor: obj.textColor
        };
    }
    return {
        gradientStart: data?.buyBtnGradientStart || dflt.gradientStart,
        gradientEnd: data?.buyBtnGradientEnd || dflt.gradientEnd,
        textColor: data?.buyBtnTextColor || dflt.textColor
    };
}
