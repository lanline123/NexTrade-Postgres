import { useMemo, type CSSProperties } from "react";
import { View, Text, Image } from "@tarojs/components";
import dayjs from "dayjs";

import { productImage } from "@/utils/format";
import { useConfigStore } from "@/store/config";
import { resolveH5BuyBtnStyle } from "@/utils/buyStyle";
import ActivityCard, { isActivityVisible } from "./ActivityCard";
import FormatPrice from "@/components/format-price/FormatPrice";
import TigLazyImage from "@/components/tig/tig-lazy-image/TigLazyImage";
import { staticResource } from "@/utils";
import { shouldShowSoldOut } from "@/utils/productSoldOut";
import { t } from "@/utils/i18n";
import type { ProductActiveStyleVO } from "@/types/product/productStyle";

import "./ProductItem.scss";

interface Props {
    item: any;
    promotionList: any;
    styleMap?: Record<number, ProductActiveStyleVO>;
    type?: string;
    showShop?: boolean;
    isCycleBuy?: boolean;
    isSubCard?: boolean;
    /** 文本对齐：1=左对齐，2=居中对齐 */
    textAlign?: number;
    /** 文字粗细：1=常规体，2=加粗体 */
    textWeight?: number;
    /** 商品名称行数：1=1行，2=2行 */
    goodsNameRow?: number;
    /** 商品标题行数（兼容字段，等价 goodsNameRow） */
    productTitleLine?: number;
    /** 是否显示商品名称 */
    showName?: boolean;
    /** 购买按钮样式：undefined=沿用旧图标，0=不显示，1~8=对应样式，-1=跟随全局（读时解析） */
    buyBtnStyle?: number;
    /** 是否显示优惠券活动；undefined 表示沿用其他页面原有行为 */
    showCoupon?: boolean;
    productTitleColor?: string;
    priceColor?: string;
    themeColor?: string;
    itemBackgroundColor?: string;
    productImageRadius?: number;
    onClickItem?: (item: any) => void;
    onClickBuy?: (item: any) => void;
    onClickShop?: (shopId: number) => void;
}

/** 商品卡片（自旧 masonry/product-item.vue 移植，瀑布流/搜索/分类列表共用） */
export default function ProductItem({
    item,
    promotionList,
    styleMap = {},
    type = "",
    isSubCard = false,
    textAlign,
    textWeight,
    goodsNameRow,
    productTitleLine,
    showName = true,
    buyBtnStyle,
    showCoupon,
    productTitleColor,
    priceColor,
    themeColor,
    itemBackgroundColor,
    productImageRadius,
    showShop,
    onClickItem,
    onClickBuy,
    onClickShop
}: Props) {
    // --pi-* 由装修配置下发（见 decorateVars），未配置时 fallback 即原有硬编码值
    const decorateVars = useMemo(() => {
        const vars: Record<string, string> = {};
        if (productTitleColor?.trim()) vars["--pi-title-color"] = productTitleColor;
        if (priceColor?.trim()) vars["--pi-price-color"] = priceColor;
        if (themeColor?.trim()) vars["--pi-theme"] = themeColor;
        if (itemBackgroundColor?.trim()) vars["--pi-item-bg"] = itemBackgroundColor;
        if (productImageRadius != null) vars["--pi-image-radius"] = `${Math.max(0, productImageRadius) * 2}rpx`;
        return vars;
    }, [productTitleColor, priceColor, themeColor, itemBackgroundColor, productImageRadius]);

    const productCardRadius = productImageRadius == null ? undefined : `${Math.max(0, productImageRadius) * 2}rpx`;
    // 跟随全局解析：buyBtnStyle=-1 时取全局样式降级到 0–8（读时、不回写）；
    // undefined/0–8 原样透传，行为与此前逐字一致。全局 iconColor 以 --general
    // 局部变量作用于按钮元素本身。
    const globalBuyStyle = useConfigStore((s) => s.buyStyle);
    const resolvedBuyBtn = resolveH5BuyBtnStyle(buyBtnStyle, globalBuyStyle);
    const effBuyBtnStyle = resolvedBuyBtn.style;
    const showBuyBtn = effBuyBtnStyle !== 0;

    const itemClasses = useMemo(() => {
        const cls: string[] = [];
        if (type === "exchange") cls.push("item-li--exchange");
        if (textAlign === 2) cls.push("item-li--align-2");
        if (textWeight === 1) cls.push("item-li--weight-1");
        const row = goodsNameRow ?? productTitleLine;
        if (row === 1) cls.push("item-li--name-row-1");
        return cls.join(" ");
    }, [type, textAlign, textWeight, goodsNameRow, productTitleLine]);

    const row = goodsNameRow ?? productTitleLine;
    const detailStyle = row === 1 || row === 2 ? ({ "-webkit-line-clamp": row, "line-clamp": row } as Record<string, string | number>) : {};

    const currentStyle = useMemo(() => {
        const productId = item?.productId;
        return productId ? (styleMap[productId] ?? null) : null;
    }, [item, styleMap]);

    // 计算是否显示售罄状态
    const showOutsale = shouldShowSoldOut(item, { isSubCard });
    // 已下架态（原版 card.vue：productStatus==0 且未售罄时角标文案为「已下架」）
    const isOffShelf = !showOutsale && Number(item?.productStatus) === 0;

    // 计算促销信息
    const currentPromotionInfo = useMemo(() => {
        const promotionItem = promotionList[item.productId];
        const activityInfo = promotionItem?.activityInfo || [];
        if (showCoupon === false) {
            return activityInfo.filter((it: any) => Number(it?.type) !== 2);
        }
        return activityInfo;
    }, [promotionList, item, showCoupon]);

    // 标签行只按「能渲染出可见标签」的条目判定存在：原版 activityInfo 只含真实促销条目，
    // 条目数判定即可见性判定；我方后端会下发 type=0 / type=8（抽奖）等无文案条目，
    // 若仍按条目数判定，无促销标签时标签行仍占 55rpx 空高、把价格推远（与原版渲染口径不符）。
    // 有可见标签时仍是原版 55rpx 标签行，无可见标签时整行不渲染、价格按原版无促销间距贴近标题。
    const visiblePromotionInfo = useMemo(
        () => currentPromotionInfo.filter((subItem: any) => isActivityVisible(subItem)),
        [currentPromotionInfo]
    );
    const hasPromotionInfo = Object.keys(promotionList).length > 0 && visiblePromotionInfo.length > 0;

    const hasGrouponActivity = useMemo(() => {
        // 判断当前的活动是否有拼团活动
        const promotionItem = currentPromotionInfo.find((it: any) => it.type === 7);
        if (!promotionItem) return false;
        // 判断当前时间是否在活动时间内
        const now = dayjs().format("YYYY-MM-DD HH:mm:ss");
        if (promotionItem.startTime > now || promotionItem.endTime < now) return false;
        return true;
    }, [currentPromotionInfo]);

    return (
        <View
            className={`item-li ${itemClasses}`}
            style={{ ...decorateVars, backgroundColor: itemBackgroundColor || undefined, borderRadius: productCardRadius } as any}
            onClick={() => onClickItem?.(item)}
        >
            <View className="photo" style={{ borderRadius: productCardRadius } as any}>
                <TigLazyImage src={productImage(item)} mode="widthFix" />
                {(showOutsale || isOffShelf) && (
                    <View className="product-status-box">
                        <View className="outsale">{showOutsale ? t("已售罄") : t("已下架")}</View>
                    </View>
                )}
                {currentStyle?.borderImage && <Image className="product-style-border" src={currentStyle.borderImage} mode="scaleToFill" />}
            </View>
            <View className="info">
                {showName && (
                    <View className="detail presale-box" style={detailStyle as any}>
                        {hasGrouponActivity && <Text className="groupon-tag">{t("拼")}</Text>}
                        {(item.saleMode && item.saleMode > 0) || item.productSaleMode ? (
                            <>
                                <Text className="presale-tag">{t("预售")}</Text>
                                <Text className="separator">|</Text>
                            </>
                        ) : null}
                        <Text>{item.productName}</Text>
                    </View>
                )}
                {hasPromotionInfo && (
                    <View className="flex activity-box">
                        {visiblePromotionInfo.map((subItem: any) => (
                            <ActivityCard key={subItem.promotionId} activityInfo={subItem} themeColor={themeColor} />
                        ))}
                    </View>
                )}

                <View className="action">
                    <View className="pricenum">
                        {type === "exchange" ? (
                            <View className="exchange-box">
                                <FormatPrice displayPrice
                                    fontStyle={{ fontSize: "32rpx" }}
                                    currencyStyle={{ fontSize: "22rpx" }}
                                    decimalsStyle={{ fontSize: "25rpx" }}
                                    priceData={item.discountsPrice}
                                />
                                <Text className="exchange-icon">+</Text>
                                <Image className="exchange-img" src={staticResource("exchange/exchange.png")} />
                                <Text className="exchange-text">{item.exchangeIntegral}</Text>
                            </View>
                        ) : (
                            <FormatPrice displayPrice
                                fontStyle={{ fontSize: "32rpx" }}
                                currencyStyle={{ fontSize: "22rpx", position: "relative", top: "3rpx" }}
                                priceData={item.price ?? item.productPrice}
                            />
                        )}
                    </View>

                    {showBuyBtn && (
                        <View
                            className={`buy_btn ${effBuyBtnStyle && effBuyBtnStyle >= 1 && effBuyBtnStyle <= 8 ? `buy_btn--style${effBuyBtnStyle}` : ""}`}
                            style={resolvedBuyBtn.iconColor ? ({ "--general": resolvedBuyBtn.iconColor } as CSSProperties) : undefined}
                            onClick={(e) => {
                                e.stopPropagation();
                                onClickBuy?.(item);
                            }}
                        >
                            {item.isCheck ? (
                                <Text className="buy_btn_text">…</Text>
                            ) : effBuyBtnStyle === 5 || effBuyBtnStyle === 6 ? (
                                <Text className="buy_btn_text">{t("购买")}</Text>
                            ) : effBuyBtnStyle === 7 || effBuyBtnStyle === 8 ? (
                                <Text className="buy_btn_text">{t("马上抢")}</Text>
                            ) : effBuyBtnStyle === 1 ? (
                                <Text className="iconfont-h5 buy_btn_icon icon-gouwuche" />
                            ) : effBuyBtnStyle === 2 ? (
                                <Text className="iconfont-h5 buy_btn_icon icon-gouwuche1" />
                            ) : effBuyBtnStyle === 3 ? (
                                <Text className="iconfont-h5 buy_btn_icon icon-jia" />
                            ) : effBuyBtnStyle === 4 ? (
                                <Text className="iconfont-h5 buy_btn_icon icon-jia1" />
                            ) : type !== "exchange" ? (
                                // 未传 buyBtnStyle 时沿用旧图标（其他场景不受影响）
                                <Text className="iconfont-h5 icon-gouwuche3 buy_btn_icon" />
                            ) : null}
                        </View>
                    )}
                </View>

                {showShop !== false && item.shop && (
                    <View
                        className="shop"
                        onClick={(e) => {
                            e.stopPropagation();
                            onClickShop?.(item.shop.shopId);
                        }}
                    >
                        <View className="shop-name">{item.shop.shopTitle}</View>
                        <View className="shop-btn">
                            {t("进店")}
                            <Text className="iconfont-h5 icon-youjiantou" />
                        </View>
                    </View>
                )}
            </View>
        </View>
    );
}
