import { useMemo } from "react";
import { View } from "@tarojs/components";

import { convertInlineRpx } from "@/utils/inlineRpxH5";

import { useConfigStore } from "@/store/config";
import { useCurrencyStore } from "@/store/currency";
import { useUserStore } from "@/store/user";
import { isB2B, isOverseas, moneyFormat } from "@/utils";
import { priceGateCopy } from "@/utils/inquiry";
import { abbreviateDisplayPrice, currencyCodeFrom, DEFAULT_PRICE_DISPLAY } from "@/utils/priceDisplay";
import { t } from "@/utils/i18n";

import "./FormatPrice.scss";

type StyleObj = Record<string, string>;

interface Props {
    priceData?: string | number | null;
    isBottom?: boolean;
    currencyFormat?: boolean;
    currencyStyle?: StyleObj;
    fontStyle?: StyleObj;
    decimalsStyle?: StyleObj;
    integerStyle?: StyleObj;
    showDecimals?: boolean;
    showText?: boolean;
    base?: boolean;
    /** 展示价标记：按后台「价格显示设置」缩写大额（≥1万显示 XX万 等），与 PC FormatPrice 同名同口径；缺省精确显示，购物车/结算/订单等对账场景不传 */
    displayPrice?: boolean;
    /** 对账价标记：与 PC formatDisplayPriceWithExact 同口径——触发缩写时渲染「缩写（符号+精确到分）」双段（如 ¥6.5万（¥65000.00），全角括号、精确段固定两位小数），未触发时与缺省精确渲染逐字一致；购物车/结算/收银台/订单/售后等对账场景传，与 displayPrice 二选一 */
    withExact?: boolean;
}

/**
 * 价格展示（自旧 format-price.vue 移植）：
 * - 海外站点按当前货币汇率换算，货币符号优先取当前货币，缺省回退后台 dollarSign
 * - B2B 且未登录 / 未实名时显示「登录查看 / 实名查看」文案
 * - displayPrice 标记时按后台「价格显示设置」缩写大额展示价（与 PC 同口径，见 utils/priceDisplay）
 * - withExact 标记时按 PC formatDisplayPriceWithExact 同口径渲染对账双段：缩写主读数 + 全角括号内精确额
 */
export default function FormatPrice({
    priceData,
    isBottom = true,
    currencyFormat = true,
    currencyStyle = {},
    fontStyle = {},
    decimalsStyle = {},
    integerStyle = {},
    showDecimals = true,
    showText = true,
    base = false,
    displayPrice = false,
    withExact = false
}: Props) {
    const dollarSign = useConfigStore((s) => s.dollarSign);
    const isIdentity = useConfigStore((s) => s.isIdentity);
    const isLogin = useConfigStore((s) => s.isLogin);
    const priceDisplay = useConfigStore((s) => s.priceDisplay);
    const currentCurrencyData = useCurrencyStore((s) => s.currentCurrencyData);
    const userInfo = useUserStore((s) => s.userInfo);

    const hasContent = (styleObj: StyleObj) => {
        return styleObj && Object.keys(styleObj).length > 0;
    };

    // H5 内联样式不经 PostCSS：rpx 字符串会被浏览器按无效值丢弃、字号回退到继承值（价格显大，
    // 2026-11-18 分类页实测定位）。组件级先行换算（与 utils/inlineRpxH5 全局收口同口径、互为兜底）。
    const convStyle = (styleObj: StyleObj): StyleObj => {
        if (process.env.TARO_ENV !== "h5" || !styleObj) return styleObj;
        const out: StyleObj = {};
        for (const [k, v] of Object.entries(styleObj)) {
            out[k] = typeof v === "string" ? convertInlineRpx(v) : v;
        }
        return out;
    };
    const currencyStyleC = useMemo(() => convStyle(currencyStyle), [currencyStyle]);
    const fontStyleC = useMemo(() => convStyle(fontStyle), [fontStyle]);
    const decimalsStyleC = useMemo(() => convStyle(decimalsStyle), [decimalsStyle]);
    const decimalStyle = {
        ...fontStyleC,
        ...(currencyStyleC.fontSize ? { fontSize: currencyStyleC.fontSize } : {}),
        ...decimalsStyleC
    };
    const integerStyleC = useMemo(() => convStyle(integerStyle), [integerStyle]);

    const b2bText = useMemo(() => {
        let str = t("登录查看");
        if (Object.keys(userInfo).length > 0) {
            str = t("实名查看");
        }
        return str;
    }, [userInfo]);

    const showB2bText = useMemo(() => {
        let value = false;
        if (isB2B() && showText) {
            if (isIdentity === 1) {
                if (Object.keys(userInfo).length === 0) {
                    value = true;
                } else if (!userInfo.isCompanyAuth) {
                    value = true;
                }
            } else if (isLogin === 1 && Object.keys(userInfo).length === 0) {
                // 登录门单独开启：仅匿名显示「登录查看」，已登录不问认证
                value = true;
            }
        }
        return value;
    }, [isIdentity, isLogin, userInfo, showText]);

    /**
     * 价格门控（第 2 期，我方新增）：后端在无看价权时把价格置空（null）。
     * 载荷价为空且价格门控开启（isIdentity 或 isLogin）、当前用户无看价权时，同样渲染
     * 门控文案而不是假的 ¥0.00——覆盖非 B2B 模式下原版展示门控不触发的
     * 场景；有价商品的 priceData 永远非 null，不受影响。
     * 注意触发须用 == null：多处卡片以 `a ?? b` 链传价，脱敏 null 会被
     * 兜底成 undefined，严格 === null 会漏触发（2026-11-20 首页/列表实证）。
     */
    const showMaskedGateText = useMemo(() => {
        if (!showText || priceData != null) return false;
        if (isIdentity === 1) {
            return Object.keys(userInfo).length === 0 || !userInfo.isCompanyAuth;
        }
        // 登录门单独开启：仅匿名且价格被脱敏时显示门控文案
        if (isLogin === 1) {
            return Object.keys(userInfo).length === 0;
        }
        return false;
    }, [isIdentity, isLogin, userInfo, showText, priceData]);

    /** 脱敏路径门控文案与详情页同口径（utils/inquiry 的 priceGateCopy）：
     * 未登录「登录后查看价格」，已登录未认证「认证后查看价格」。 */
    const maskedGateText = useMemo(() => {
        return priceGateCopy(Object.keys(userInfo).length === 0 ? "login" : "certification").text;
    }, [userInfo]);

    const showGateText = showB2bText || showMaskedGateText;

    const currency = useMemo(() => {
        const baseCurrency = dollarSign || "￥";
        if (base) return baseCurrency;
        return isOverseas() ? currentCurrencyData.symbol || baseCurrency : baseCurrency;
    }, [base, dollarSign, currentCurrencyData.symbol]);

    const price = useMemo(() => {
        let num = typeof priceData === "number" ? String(priceData) : priceData;
        if (!num) {
            return {
                integer: "0",
                decimals: ".00",
                suffix: "",
                exact: ""
            };
        }
        if (!base && isOverseas() && currentCurrencyData.rate) {
            num = moneyFormat((Number(num) * Number(currentCurrencyData.rate)).toString());
        }
        // 价格缩写（displayPrice / withExact 标记，后台「价格显示设置」，与 PC 同口径）：
        // 在换算后金额上按当前币种阶梯缩写，未触发时回落下面的精确拆分；
        // 门控文案分支在渲染层先行，本处只处理有价数字，不影响门控逻辑。
        // withExact 触发时另带精确段（换算后金额固定两位小数、无千分位，与 PC
        // formatPrice 的数字部分逐字一致），渲染层拼成「缩写（符号+精确）」双段；
        // 缩写与精确恰好同串时（如除数为 1 且后缀为空）按 PC 的 display===exact
        // 守卫回落单段精确，不渲染重复括号段。
        if (displayPrice || withExact) {
            const code = base
                ? "CNY"
                : isOverseas()
                  ? currencyCodeFrom(currentCurrencyData.name, currentCurrencyData.symbol)
                  : "CNY";
            const abbr = abbreviateDisplayPrice(Number(num), code, priceDisplay ?? DEFAULT_PRICE_DISPLAY);
            if (abbr) {
                if (!withExact) return { ...abbr, exact: "" };
                const exactText = Number(num).toFixed(2);
                if (`${abbr.integer}${abbr.decimals}${abbr.suffix}` !== exactText) {
                    return { ...abbr, exact: exactText };
                }
            }
        }
        // 检测小数点和后续数字
        const match = num.match(/^(\d+)(\.\d+)?$/);
        if (match) {
            const integerPart = match[1] || "0";
            const rawDecimal = match[2] || ".00";
            const decimalPart = rawDecimal.length < 3 ? rawDecimal.padEnd(3, "0") : rawDecimal;
            return {
                integer: integerPart,
                decimals: decimalPart,
                suffix: "",
                exact: ""
            };
        } else {
            return {
                integer: "0",
                decimals: ".00",
                suffix: "",
                exact: ""
            };
        }
    }, [priceData, base, currentCurrencyData.rate, currentCurrencyData.name, currentCurrencyData.symbol, displayPrice, withExact, priceDisplay]);

    // 原版 #ifdef MP-WEIXIN 的 1–2rpx 微调（.util padding-right 5rpx、bottom 态 padding-bottom 3rpx）：
    // Taro 侧按本仓既有口径用 TARO_ENV 加修饰类承载（见 FormatPrice.scss），H5 不加类、零变化
    const isWeapp = process.env.TARO_ENV === "weapp";

    return (
        <View className={`price-content ${isBottom ? "bottom" : ""}${isWeapp ? " price-content--weapp" : ""}`}>
            {showGateText ? (
                <View className="line1 b2b-text">{showB2bText ? b2bText : maskedGateText}</View>
            ) : (
                <>
                    {currencyFormat && (
                        <View
                            className="num util"
                            style={(hasContent(currencyStyleC) ? currencyStyleC : fontStyleC) as any}
                        >
                            {currency}
                        </View>
                    )}
                    <View className="num integer" style={(hasContent(integerStyleC) ? integerStyleC : fontStyleC) as any}>
                        {price.integer}
                    </View>
                    {showDecimals && price.decimals ? (
                        <View className="num decimal" style={decimalStyle}>
                            {price.decimals}
                        </View>
                    ) : null}
                    {price.suffix ? (
                        <View className="num suffix" style={decimalStyle}>
                            {price.suffix}
                        </View>
                    ) : null}
                    {price.exact ? (
                        <View className="num exact" style={fontStyleC as any}>
                            {`（${currency}${price.exact}）`}
                        </View>
                    ) : null}
                </>
            )}
        </View>
    );
}
