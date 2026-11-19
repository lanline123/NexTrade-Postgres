import { Text, View } from "@tarojs/components";

import FormatPrice from "@/components/format-price/FormatPrice";
import { resolveInquiryState } from "@/utils/inquiry";
import { t } from "@/utils/i18n";

import "./InquiryPrice.scss";

type StyleObj = Record<string, string>;

interface Props {
    /** 商品载荷（读 isInquiry / inquiryState / minPricedPrice / priceVisible） */
    product?: Record<string, any> | null;
    /** 直接价格（搜索列表的 price 键、周期购 cyclePrice 等）：未传时取 product.productPrice */
    priceData?: string | number | null;
    fontStyle?: StyleObj;
    currencyStyle?: StyleObj;
    decimalsStyle?: StyleObj;
}

/**
 * 卡片价格位（询价线第 1/2/3 期，与 PC 端 ProductCard 同口径）：
 * - 全询价：价格位渲染「询价」，不露占位价；
 * - 部分询价：有价型号最低价 + 「起」+ 弱标记「部分型号询价」；
 * - 价格门控：后端已把价格置空（null），FormatPrice 内部按门控文案渲染，
 *   本组件只负责不把 null 兜底成 0 价、且门控时不带「起」。
 */
export default function InquiryPrice({ product, priceData, fontStyle = {}, currencyStyle = {}, decimalsStyle = {} }: Props) {
    const state = resolveInquiryState(product);
    if (state === "all") {
        return (
            <View className="inquiry-price-text" style={fontStyle as any}>
                {t("询价")}
            </View>
        );
    }
    const gated = (product as any)?.priceVisible === false;
    const price =
        state === "partial"
            ? ((product as any)?.minPricedPrice ?? priceData ?? (product as any)?.productPrice)
            : (priceData ?? (product as any)?.productPrice ?? (product as any)?.price);
    return (
        <View className="inquiry-price-box">
            <FormatPrice fontStyle={fontStyle} currencyStyle={currencyStyle} decimalsStyle={decimalsStyle} priceData={price} />
            {state === "partial" && !gated ? <Text className="inquiry-price-from">{t("起")}</Text> : null}
            {state === "partial" ? <View className="inquiry-price-note">{t("部分型号询价")}</View> : null}
        </View>
    );
}
