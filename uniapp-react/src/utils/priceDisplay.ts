/**
 * 全站价格显示格式（后台「价格显示设置」priceDisplay 配置）——移动端消费侧。
 *
 * 与 PC（pc-semi/src/lib/currency.ts 的 formatDisplayPrice / splitDisplayPrice）
 * 同口径移植：大额展示价按当前币种单位阶梯缩写（人民币 万/亿、美元 K/M/B、
 * 印度卢比 K/L/Cr 等），规则由后台逐币种自由配置，经公开端点
 * GET /api/common/config/priceDisplay 下发，基础配置拉取时一并写入
 * config store（见 store/config 的 loadBaseConfigData，随回前台刷新同路更新）。
 *
 * 仅展示场景消费（FormatPrice 的 displayPrice 标记，与 PC 同名同口径）；
 * 购物车 / 结算 / 订单等对账金额不经此路，始终精确到分。
 * 配置缺失/未拉到时用内置默认（与后端默认、PC 内置默认一致），行为零漂移。
 */

export interface PriceUnitRule {
    /** 启用门槛：换算后金额 ≥ at 才用这条规则 */
    at: number;
    /** 除数：显示值 = 金额 / div */
    div: number;
    /** 单位后缀（万 / 亿 / K / M / L / Cr …，后台自由填写） */
    suffix: string;
}

export interface PriceDisplayConfig {
    enabled: boolean;
    /** 缩写后保留的小数位数（0–4） */
    decimals: number;
    /** 去掉小数尾零（100.00万 → 100万） */
    trimZeros: boolean;
    /** 币种代码 → 单位阶梯；空数组 = 该币种不缩写；缺失 = 用通用 K/M/B */
    currencies: Record<string, PriceUnitRule[]>;
}

const WESTERN_LADDER: PriceUnitRule[] = [
    { at: 10000, div: 1000, suffix: "K" },
    { at: 1000000, div: 1000000, suffix: "M" },
    { at: 1000000000, div: 1000000000, suffix: "B" }
];

/** 接口不可用/未拉到时的内置兜底（与后端默认配置、PC DEFAULT_PRICE_DISPLAY 一致）。 */
export const DEFAULT_PRICE_DISPLAY: PriceDisplayConfig = {
    enabled: true,
    decimals: 2,
    trimZeros: true,
    currencies: {
        CNY: [
            { at: 10000, div: 10000, suffix: "万" },
            { at: 100000000, div: 100000000, suffix: "亿" }
        ],
        HKD: [
            { at: 10000, div: 10000, suffix: "万" },
            { at: 100000000, div: 100000000, suffix: "億" }
        ],
        TWD: [
            { at: 10000, div: 10000, suffix: "萬" },
            { at: 100000000, div: 100000000, suffix: "億" }
        ],
        JPY: [
            { at: 10000, div: 10000, suffix: "万" },
            { at: 100000000, div: 100000000, suffix: "億" }
        ],
        KRW: [
            { at: 10000, div: 10000, suffix: "만" },
            { at: 100000000, div: 100000000, suffix: "억" }
        ],
        INR: [
            { at: 10000, div: 1000, suffix: "K" },
            { at: 100000, div: 100000, suffix: "L" },
            { at: 10000000, div: 10000000, suffix: "Cr" }
        ],
        RUB: [
            { at: 10000, div: 1000, suffix: " тыс." },
            { at: 1000000, div: 1000000, suffix: " млн" },
            { at: 1000000000, div: 1000000000, suffix: " млрд" }
        ],
        IDR: [
            { at: 10000, div: 1000, suffix: "K" },
            { at: 1000000, div: 1000000, suffix: "Jt" },
            { at: 1000000000, div: 1000000000, suffix: "M" }
        ],
        VND: [
            { at: 100000, div: 1000, suffix: "K" },
            { at: 1000000, div: 1000000, suffix: "Tr" },
            { at: 1000000000, div: 1000000000, suffix: "Tỷ" }
        ],
        USD: WESTERN_LADDER,
        EUR: WESTERN_LADDER,
        GBP: WESTERN_LADDER,
        AUD: WESTERN_LADDER,
        CAD: WESTERN_LADDER,
        SGD: WESTERN_LADDER,
        CHF: WESTERN_LADDER,
        NZD: WESTERN_LADDER,
        MYR: WESTERN_LADDER,
        PHP: WESTERN_LADDER,
        PLN: WESTERN_LADDER,
        SEK: WESTERN_LADDER,
        NOK: WESTERN_LADDER,
        DKK: WESTERN_LADDER,
        ZAR: WESTERN_LADDER,
        MXN: WESTERN_LADDER,
        SAR: WESTERN_LADDER,
        AED: WESTERN_LADDER,
        QAR: WESTERN_LADDER,
        KWD: WESTERN_LADDER,
        BRL: WESTERN_LADDER,
        THB: WESTERN_LADDER,
        TRY: WESTERN_LADDER,
        ILS: WESTERN_LADDER,
        CZK: WESTERN_LADDER,
        HUF: WESTERN_LADDER
    }
};

/** 清洗一份下发的价格显示配置（与 PC sanitizePriceDisplay 同口径）；非法输入返回 null。 */
export function sanitizePriceDisplay(input: unknown): PriceDisplayConfig | null {
    if (!input || typeof input !== "object") return null;
    const raw = input as Record<string, unknown>;
    const currencies: Record<string, PriceUnitRule[]> = {};
    if (raw.currencies && typeof raw.currencies === "object") {
        for (const [code, rules] of Object.entries(raw.currencies as Record<string, unknown>)) {
            if (!Array.isArray(rules)) continue;
            const clean: PriceUnitRule[] = [];
            for (const r of rules) {
                if (!r || typeof r !== "object") continue;
                const at = Number((r as PriceUnitRule).at);
                const div = Number((r as PriceUnitRule).div);
                const suffix = String((r as PriceUnitRule).suffix ?? "").slice(0, 8);
                if (Number.isFinite(at) && at > 0 && Number.isFinite(div) && div > 0) {
                    clean.push({ at, div, suffix });
                }
            }
            clean.sort((a, b) => a.at - b.at);
            currencies[String(code).toUpperCase().slice(0, 8)] = clean.slice(0, 6);
            if (Object.keys(currencies).length >= 64) break;
        }
    }
    const decimals = Number(raw.decimals);
    return {
        enabled: raw.enabled === undefined ? true : Boolean(raw.enabled),
        decimals: Number.isFinite(decimals) ? Math.min(4, Math.max(0, Math.floor(decimals))) : 2,
        trimZeros: raw.trimZeros === undefined ? true : Boolean(raw.trimZeros),
        currencies
    };
}

/** 币种名称/符号 → ISO 代码（与 PC currency.ts 同表移植；移动端 CurrencyItem 无代码字段，按名称优先匹配）。 */
const NAME_TO_CODE: Record<string, string> = {
    美元: "USD",
    人民币: "CNY",
    欧元: "EUR",
    英镑: "GBP",
    日元: "JPY",
    瑞士法郎: "CHF",
    加拿大元: "CAD",
    澳大利亚元: "AUD",
    新加坡元: "SGD",
    港元: "HKD",
    港币: "HKD",
    韩元: "KRW",
    新台币: "TWD",
    台币: "TWD",
    印度卢比: "INR",
    卢比: "INR",
    俄罗斯卢布: "RUB",
    卢布: "RUB",
    巴西雷亚尔: "BRL",
    雷亚尔: "BRL",
    马来西亚林吉特: "MYR",
    林吉特: "MYR",
    菲律宾比索: "PHP",
    比索: "PHP",
    波兰兹罗提: "PLN",
    土耳其里拉: "TRY",
    里拉: "TRY",
    南非兰特: "ZAR",
    兰特: "ZAR",
    阿联酋迪拉姆: "AED",
    迪拉姆: "AED",
    沙特里亚尔: "SAR",
    泰铢: "THB",
    越南盾: "VND",
    印尼盾: "IDR",
    印度尼西亚盾: "IDR",
    新西兰元: "NZD",
    瑞典克朗: "SEK",
    挪威克朗: "NOK",
    丹麦克朗: "DKK",
    墨西哥比索: "MXN",
    卡塔尔里亚尔: "QAR",
    科威特第纳尔: "KWD",
    第纳尔: "KWD",
    捷克克朗: "CZK",
    匈牙利福林: "HUF",
    以色列谢克尔: "ILS"
};

const SYMBOL_TO_CODE: Record<string, string> = {
    $: "USD",
    "￥": "CNY",
    "¥": "JPY",
    "€": "EUR",
    "£": "GBP",
    CHF: "CHF",
    "C$": "CAD",
    "A$": "AUD",
    "S$": "SGD",
    "HK$": "HKD",
    "₩": "KRW",
    "NT$": "TWD",
    "₹": "INR",
    "₽": "RUB",
    "R$": "BRL",
    RM: "MYR",
    "₱": "PHP",
    "zł": "PLN",
    "₺": "TRY",
    "฿": "THB",
    "₫": "VND",
    Rp: "IDR",
    "NZ$": "NZD"
};

export function currencyCodeFrom(name?: string | null, symbol?: string | null): string {
    const byName = NAME_TO_CODE[String(name || "").trim()];
    if (byName) return byName;
    return SYMBOL_TO_CODE[String(symbol || "").trim()] || "CNY";
}

export interface DisplayPriceParts {
    integer: string;
    /** 小数段（含点，如 ".46"）；缩写后无小数时为 ""（与 PC splitDisplayPrice 同口径） */
    decimals: string;
    /** 单位后缀（万 / 亿 / K / M …） */
    suffix: string;
}

/**
 * 对已换算金额按币种阶梯缩写拆分（与 PC abbreviateAmount + splitDisplayPrice 同口径）：
 * 触发缩写返回三段；未触发（未启用 / 金额为 0 / 未达门槛 / 该币种规则为空）返回 null，
 * 调用方回落精确显示。
 */
export function abbreviateDisplayPrice(
    converted: number,
    code: string,
    cfg: PriceDisplayConfig
): DisplayPriceParts | null {
    if (!cfg.enabled || !Number.isFinite(converted) || converted === 0) return null;
    const stored = cfg.currencies[code];
    const ladder = stored === undefined ? WESTERN_LADDER : stored;
    if (!ladder.length) return null;
    const abs = Math.abs(converted);
    let rule: PriceUnitRule | null = null;
    for (const r of ladder) {
        if (abs >= r.at && (!rule || r.at > rule.at)) rule = r;
    }
    if (!rule) return null;
    let text = (abs / rule.div).toFixed(cfg.decimals);
    if (cfg.trimZeros && text.includes(".")) text = text.replace(/\.?0+$/, "");
    const signed = `${converted < 0 ? "-" : ""}${text}`;
    const dot = signed.indexOf(".");
    return {
        integer: dot >= 0 ? signed.slice(0, dot) : signed,
        decimals: dot >= 0 ? signed.slice(dot) : "",
        suffix: rule.suffix
    };
}
