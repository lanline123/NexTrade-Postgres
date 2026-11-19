import { useMemo } from "react";
import { View } from "@tarojs/components";

import { convertInlineRpx } from "@/utils/inlineRpxH5";

import { useConfigStore } from "@/store/config";
import { useCurrencyStore } from "@/store/currency";
import { useUserStore } from "@/store/user";
import { isB2B, isOverseas, moneyFormat } from "@/utils";
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
}

/**
 * 价格展示（自旧 format-price.vue 移植）：
 * - 海外站点按当前货币汇率换算，货币符号优先取当前货币，缺省回退后台 dollarSign
 * - B2B 且未登录 / 未实名时显示「登录查看 / 实名查看」文案
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
    base = false
}: Props) {
    const dollarSign = useConfigStore((s) => s.dollarSign);
    const isIdentity = useConfigStore((s) => s.isIdentity);
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
        if (isB2B() && showText && isIdentity === 1) {
            if (Object.keys(userInfo).length === 0) {
                value = true;
            } else if (!userInfo.isCompanyAuth) {
                value = true;
            }
        }
        return value;
    }, [isIdentity, userInfo, showText]);

    /**
     * 价格门控（第 2 期，我方新增）：后端在无看价权时把价格置空（null）。
     * 载荷价为 null 且 isIdentity 开启、当前用户无看价权时，同样渲染
     * 门控文案而不是假的 ¥0.00——覆盖非 B2B 模式下原版展示门控不触发的
     * 场景；有价商品的 priceData 永远非 null，不受影响。
     */
    const showMaskedGateText = useMemo(() => {
        if (!showText || priceData !== null || isIdentity !== 1) return false;
        return Object.keys(userInfo).length === 0 || !userInfo.isCompanyAuth;
    }, [isIdentity, userInfo, showText, priceData]);

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
                decimals: ".00"
            };
        }
        if (!base && isOverseas() && currentCurrencyData.rate) {
            num = moneyFormat((Number(num) * Number(currentCurrencyData.rate)).toString());
        }
        // 检测小数点和后续数字
        const match = num.match(/^(\d+)(\.\d+)?$/);
        if (match) {
            const integerPart = match[1] || "0";
            const rawDecimal = match[2] || ".00";
            const decimalPart = rawDecimal.length < 3 ? rawDecimal.padEnd(3, "0") : rawDecimal;
            return {
                integer: integerPart,
                decimals: decimalPart
            };
        } else {
            return {
                integer: "0",
                decimals: ".00"
            };
        }
    }, [priceData, base, currentCurrencyData.rate]);

    // 原版 #ifdef MP-WEIXIN 的 1–2rpx 微调（.util padding-right 5rpx、bottom 态 padding-bottom 3rpx）：
    // Taro 侧按本仓既有口径用 TARO_ENV 加修饰类承载（见 FormatPrice.scss），H5 不加类、零变化
    const isWeapp = process.env.TARO_ENV === "weapp";

    return (
        <View className={`price-content ${isBottom ? "bottom" : ""}${isWeapp ? " price-content--weapp" : ""}`}>
            {showGateText ? (
                <View className="line1 b2b-text">{b2bText}</View>
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
                    {showDecimals && (
                        <View className="num decimal" style={(hasContent(decimalsStyleC) ? decimalsStyleC : fontStyleC) as any}>
                            {price.decimals}
                        </View>
                    )}
                </>
            )}
        </View>
    );
}
