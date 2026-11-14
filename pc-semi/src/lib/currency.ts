/**
 * 货币显示（对齐旧版 view/pc components/format/Price.vue + utils/format.ts priceFormat）。
 *
 * - 符号：跨境站取 cookie `currentCurrencyData.symbol`，否则后台 base 配置 `dollarSign`（SSR 注入）
 * - 换算：`amount * rate`（rate 来自同一 cookie），与旧版 Price.vue 完全一致
 * - SSR：layout bootstrap 调 setServerCurrency(dollarSign) 注入默认符号；客户端读 document.cookie。
 *   服务端 cookies() 与 document.cookie 同源同值，SSR 与 hydration 结果一致。
 */

interface CurrencyCookieData {
  id?: number;
  name?: string;
  symbol?: string;
  rate?: string | number;
}

let SSR_SYMBOL = '';
let SSR_RATE = 1;
let SSR_CODE = '';

/** layout 服务端 bootstrap 注入后台 base 配置的默认货币符号与换币 cookie 快照 */
export function setServerCurrency(
  symbol?: string | null,
  rate?: string | number | null,
  code?: string | null,
) {
  if (symbol) SSR_SYMBOL = symbol;
  const parsedRate = Number(rate);
  if (Number.isFinite(parsedRate) && parsedRate > 0) {
    SSR_RATE = parsedRate;
  }
  if (code) SSR_CODE = code;
}

function readCookie(name: string): string {
  if (typeof document === 'undefined') return '';
  const escaped = name.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  const match = document.cookie.match(new RegExp(`(?:^|; )${escaped}=([^;]*)`));
  return match ? match[1] : '';
}

export function readCurrencyCookie(): CurrencyCookieData | null {
  const raw = readCookie('currentCurrencyData');
  if (!raw) return null;
  try {
    const parsed = JSON.parse(decodeURIComponent(raw)) as CurrencyCookieData;
    return parsed && typeof parsed === 'object' ? parsed : null;
  } catch {
    return null;
  }
}

/** 价格已自带货币符号时透传（但平台基准人民币的 `¥/￥` 前缀要先剥掉再换算） */
const HAS_CURRENCY_SIGN = /[¥￥$€£]/;
const CNY_PREFIX = /^\s*[¥￥]\s*/;

/** 当前币种符号与汇率（SSR 用 layout 注入的快照，客户端读 cookie）。 */
function resolveCurrency(): { symbol: string; rate: number } {
  const cookie = typeof document !== 'undefined' ? readCurrencyCookie() : null;
  const symbol = (typeof document !== 'undefined' ? cookie?.symbol : SSR_SYMBOL) || SSR_SYMBOL || '¥';
  const rate = Number(typeof document !== 'undefined' ? cookie?.rate : SSR_RATE);
  return { symbol, rate: Number.isFinite(rate) && rate > 0 ? rate : 1 };
}

/** 币种名称/符号 → ISO 代码（JSON-LD / 结构化数据用；按名称优先，避免 ¥/JPY 与 ￥/CNY 混淆）。 */
const NAME_TO_CODE: Record<string, string> = {
  美元: 'USD',
  人民币: 'CNY',
  欧元: 'EUR',
  英镑: 'GBP',
  日元: 'JPY',
  瑞士法郎: 'CHF',
  加拿大元: 'CAD',
  澳大利亚元: 'AUD',
  新加坡元: 'SGD',
  港元: 'HKD',
  港币: 'HKD',
  韩元: 'KRW',
  新台币: 'TWD',
  台币: 'TWD',
  印度卢比: 'INR',
  卢比: 'INR',
  俄罗斯卢布: 'RUB',
  卢布: 'RUB',
  巴西雷亚尔: 'BRL',
  雷亚尔: 'BRL',
  马来西亚林吉特: 'MYR',
  林吉特: 'MYR',
  菲律宾比索: 'PHP',
  比索: 'PHP',
  波兰兹罗提: 'PLN',
  土耳其里拉: 'TRY',
  里拉: 'TRY',
  南非兰特: 'ZAR',
  兰特: 'ZAR',
  阿联酋迪拉姆: 'AED',
  迪拉姆: 'AED',
  沙特里亚尔: 'SAR',
  泰铢: 'THB',
  越南盾: 'VND',
  印尼盾: 'IDR',
  印度尼西亚盾: 'IDR',
  新西兰元: 'NZD',
  瑞典克朗: 'SEK',
  挪威克朗: 'NOK',
  丹麦克朗: 'DKK',
  墨西哥比索: 'MXN',
  卡塔尔里亚尔: 'QAR',
  科威特第纳尔: 'KWD',
  第纳尔: 'KWD',
  捷克克朗: 'CZK',
  匈牙利福林: 'HUF',
  以色列谢克尔: 'ILS',
};

const SYMBOL_TO_CODE: Record<string, string> = {
  $: 'USD',
  '￥': 'CNY',
  '¥': 'JPY',
  '€': 'EUR',
  '£': 'GBP',
  CHF: 'CHF',
  'C$': 'CAD',
  'A$': 'AUD',
  'S$': 'SGD',
  'HK$': 'HKD',
  '₩': 'KRW',
  'NT$': 'TWD',
  '₹': 'INR',
  '₽': 'RUB',
  'R$': 'BRL',
  RM: 'MYR',
  '₱': 'PHP',
  'zł': 'PLN',
  '₺': 'TRY',
  '฿': 'THB',
  '₫': 'VND',
  Rp: 'IDR',
  'NZ$': 'NZD',
};

export function currencyCodeFrom(name?: string | null, symbol?: string | null): string {
  const byName = NAME_TO_CODE[String(name || '').trim()];
  if (byName) return byName;
  return SYMBOL_TO_CODE[String(symbol || '').trim()] || 'CNY';
}

/** 当前币种的 ISO 代码（客户端读 cookie；SSR 用 layout 注入的快照代码）。 */
export function currencyCode(): string {
  const cookie = typeof document !== 'undefined' ? readCurrencyCookie() : null;
  if (cookie) return currencyCodeFrom(cookie.name, cookie.symbol);
  return SSR_CODE || 'CNY';
}

/** 仅取符号（价格输入框、独立 `.util` 前缀等场景）。 */
export function currencySymbol(): string {
  return resolveCurrency().symbol;
}

/** 只取换算后的数字（两位小数、不带符号），供保留旧版 `.util`/`.num` 三段结构的组件使用。 */
export function formatAmount(value: unknown): string {
  if (value === undefined || value === null || value === '') return '';
  const text = String(value).replace(CNY_PREFIX, '');
  const num = Number(extractNumeric(text));
  if (!Number.isFinite(num)) return '';
  const { rate } = resolveCurrency();
  return (num * rate).toFixed(2);
}

/** 从「¥123.00」「$123.00」等文本里取出数字；纯数字原样返回。 */
function extractNumeric(text: string): string {
  const match = text.match(/-?\d+(?:,\d{3})*(?:\.\d+)?/);
  return match ? match[0].replace(/,/g, '') : text;
}

/**
 * 格式化价格：符号 + 汇率换算 + 两位小数。
 * 返回纯文本（旧版 Price.vue 是 DOM 结构，新版调用处按需拆分）。
 *
 * 装修数据里管理员录入的价格是「¥123.00」这种基准人民币预格式化文本；
 * 去掉 `¥/￥` 后照常按当前币种换算，不能直接透传，否则换币不生效。
 */
export function formatPrice(value: unknown): string {
  if (value === undefined || value === null || value === '') return '';
  let text = String(value);
  if (CNY_PREFIX.test(text)) {
    text = text.replace(CNY_PREFIX, '');
  } else if (HAS_CURRENCY_SIGN.test(text)) {
    return text;
  }
  const num = Number(extractNumeric(text));
  if (!Number.isFinite(num)) return String(value);
  const { symbol, rate } = resolveCurrency();
  return `${symbol}${(num * rate).toFixed(2)}`;
}

/** 三段式价格（符号 / 整数 / 小数），供保留旧版排版结构的组件使用。 */
export function splitCurrencyPrice(value: unknown): {
  symbol: string;
  integer: string;
  decimals: string;
} {
  const formatted = formatPrice(value);
  const match = formatted.match(/^([^\d\s-]*?)\s*(-?[\d,]+(?:\.(\d+))?)$/);
  if (!match) return { symbol: '', integer: formatted, decimals: '' };
  const [integer, rawDecimals = ''] = match[2].replace(/,/g, '').split('.');
  const decimals = `.${(rawDecimals || match[3] || '00').padEnd(2, '0').slice(0, 2)}`;
  return { symbol: match[1], integer, decimals };
}

/* ------------------------------------------------------------------ */
/* 全站价格显示格式（后台「价格显示设置」priceDisplay 配置）            */
/*                                                                     */
/* 大额价格按当前币种的单位阶梯缩写：人民币 万/亿、美元 K/M/B、         */
/* 印度卢比 K/L/Cr 等。后缀与除数由后台逐币种自由配置，这里只负责      */
/* 消费配置并格式化。仅展示场景使用 formatDisplayPrice /               */
/* splitDisplayPrice；购物车/结算/订单等对账金额继续用 formatPrice     */
/* 保持精确。配置由 layout 服务端预取（common/config/priceDisplay）     */
/* 注入 bootstrap，SSR 与客户端共用同一份。                            */
/* ------------------------------------------------------------------ */

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
  { at: 10000, div: 1000, suffix: 'K' },
  { at: 1000000, div: 1000000, suffix: 'M' },
  { at: 1000000000, div: 1000000000, suffix: 'B' },
];

/** 接口不可用时的内置兜底（与后端默认配置保持一致的主要币种部分）。 */
export const DEFAULT_PRICE_DISPLAY: PriceDisplayConfig = {
  enabled: true,
  decimals: 2,
  trimZeros: true,
  currencies: {
    CNY: [
      { at: 10000, div: 10000, suffix: '万' },
      { at: 100000000, div: 100000000, suffix: '亿' },
    ],
    HKD: [
      { at: 10000, div: 10000, suffix: '万' },
      { at: 100000000, div: 100000000, suffix: '億' },
    ],
    TWD: [
      { at: 10000, div: 10000, suffix: '萬' },
      { at: 100000000, div: 100000000, suffix: '億' },
    ],
    JPY: [
      { at: 10000, div: 10000, suffix: '万' },
      { at: 100000000, div: 100000000, suffix: '億' },
    ],
    KRW: [
      { at: 10000, div: 10000, suffix: '만' },
      { at: 100000000, div: 100000000, suffix: '억' },
    ],
    INR: [
      { at: 10000, div: 1000, suffix: 'K' },
      { at: 100000, div: 100000, suffix: 'L' },
      { at: 10000000, div: 10000000, suffix: 'Cr' },
    ],
    RUB: [
      { at: 10000, div: 1000, suffix: ' тыс.' },
      { at: 1000000, div: 1000000, suffix: ' млн' },
      { at: 1000000000, div: 1000000000, suffix: ' млрд' },
    ],
    IDR: [
      { at: 10000, div: 1000, suffix: 'K' },
      { at: 1000000, div: 1000000, suffix: 'Jt' },
      { at: 1000000000, div: 1000000000, suffix: 'M' },
    ],
    VND: [
      { at: 100000, div: 1000, suffix: 'K' },
      { at: 1000000, div: 1000000, suffix: 'Tr' },
      { at: 1000000000, div: 1000000000, suffix: 'Tỷ' },
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
    HUF: WESTERN_LADDER,
  },
};

let priceDisplayState: PriceDisplayConfig = DEFAULT_PRICE_DISPLAY;
let priceDisplayHydrated = false;

function sanitizePriceDisplay(input: unknown): PriceDisplayConfig | null {
  if (!input || typeof input !== 'object') return null;
  const raw = input as Record<string, unknown>;
  const currencies: Record<string, PriceUnitRule[]> = {};
  if (raw.currencies && typeof raw.currencies === 'object') {
    for (const [code, rules] of Object.entries(raw.currencies as Record<string, unknown>)) {
      if (!Array.isArray(rules)) continue;
      const clean: PriceUnitRule[] = [];
      for (const r of rules) {
        if (!r || typeof r !== 'object') continue;
        const at = Number((r as any).at);
        const div = Number((r as any).div);
        const suffix = String((r as any).suffix ?? '').slice(0, 8);
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
    currencies,
  };
}

/** layout 服务端 bootstrap 注入后台价格显示配置（SSR 与客户端各调一次，内容一致）。 */
export function setPriceDisplayConfig(input: unknown): void {
  const clean = sanitizePriceDisplay(input);
  if (clean) {
    priceDisplayState = clean;
    priceDisplayHydrated = true;
  }
}

/** 当前生效的价格显示配置（客户端首次使用时从 layout 内联 bootstrap 懒加载）。 */
export function priceDisplayConfig(): PriceDisplayConfig {
  if (!priceDisplayHydrated && typeof document !== 'undefined') {
    priceDisplayHydrated = true;
    try {
      const raw = document.getElementById('pc-header-bootstrap')?.textContent;
      const parsed = raw ? (JSON.parse(raw) as any) : null;
      const clean = sanitizePriceDisplay(parsed?.priceDisplay);
      if (clean) priceDisplayState = clean;
    } catch {
      /* bootstrap 缺失时用内置默认 */
    }
  }
  return priceDisplayState;
}

/** 把已换算金额按当前币种阶梯缩写；不触发缩写时返回 null。 */
function abbreviateAmount(converted: number): { numText: string; suffix: string } | null {
  const cfg = priceDisplayConfig();
  if (!cfg.enabled || !Number.isFinite(converted) || converted === 0) return null;
  const stored = cfg.currencies[currencyCode()];
  const ladder = stored === undefined ? WESTERN_LADDER : stored;
  if (!ladder.length) return null;
  const abs = Math.abs(converted);
  let rule: PriceUnitRule | null = null;
  for (const r of ladder) {
    if (abs >= r.at && (!rule || r.at > rule.at)) rule = r;
  }
  if (!rule) return null;
  let text = (abs / rule.div).toFixed(cfg.decimals);
  if (cfg.trimZeros && text.includes('.')) text = text.replace(/\.?0+$/, '');
  return { numText: `${converted < 0 ? '-' : ''}${text}`, suffix: rule.suffix };
}

/**
 * 展示用价格格式化：与 formatPrice 相同的符号/换算，金额达到门槛时按
 * 当前币种单位阶梯缩写（¥100万 / $1.5M / ₹2.3Cr）。
 * 仅用于商品展示场景；对账金额请用 formatPrice。
 */
export function formatDisplayPrice(value: unknown): string {
  if (value === undefined || value === null || value === '') return '';
  let text = String(value);
  if (CNY_PREFIX.test(text)) {
    text = text.replace(CNY_PREFIX, '');
  } else if (HAS_CURRENCY_SIGN.test(text)) {
    return text;
  }
  const num = Number(extractNumeric(text));
  if (!Number.isFinite(num)) return String(value);
  const { symbol, rate } = resolveCurrency();
  const converted = num * rate;
  const abbr = abbreviateAmount(converted);
  if (abbr) return `${symbol}${abbr.numText}${abbr.suffix}`;
  return `${symbol}${converted.toFixed(2)}`;
}

/**
 * 结算类页面用：缩写在前、精确金额括号在后（¥6.5万（¥65000.00)）。
 * 未达到缩写门槛时只显示精确金额，不加括号。购物车/结算/收银台/
 * 订单/售后等需要核对金额的页面用这个；纯展示场景用 formatDisplayPrice。
 */
export function formatDisplayPriceWithExact(value: unknown): string {
  const exact = formatPrice(value);
  if (!exact) return exact;
  const display = formatDisplayPrice(value);
  if (!display || display === exact) return exact;
  return `${display}（${exact}）`;
}

/** 展示用三段式价格（符号 / 数字 / 后缀），供大字价格排版使用；suffix 为单位（万/K…）。 */
export function splitDisplayPrice(value: unknown): {
  symbol: string;
  integer: string;
  decimals: string;
  suffix: string;
} {
  const formatted = formatDisplayPrice(value);
  const match = formatted.match(/^([^\d\s-]*?)\s*(-?[\d,]+(?:\.(\d+))?)(.*)$/);
  if (!match) return { symbol: '', integer: formatted, decimals: '', suffix: '' };
  const numeric = match[2].replace(/,/g, '');
  const dot = numeric.indexOf('.');
  const integer = dot >= 0 ? numeric.slice(0, dot) : numeric;
  const frac = dot >= 0 ? numeric.slice(dot + 1) : match[3] || '';
  // 与 splitCurrencyPrice 对齐：未缩写时小数固定两位展示；缩写时按配置小数位原样
  const abbreviated = Boolean(match[4]);
  const decimals = abbreviated ? (frac ? `.${frac}` : '') : `.${(frac || '00').padEnd(2, '0').slice(0, 2)}`;
  return { symbol: match[1], integer, decimals, suffix: match[4] || '' };
}
