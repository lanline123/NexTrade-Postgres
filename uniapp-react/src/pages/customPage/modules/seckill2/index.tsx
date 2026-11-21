import { useEffect, useMemo, useState } from "react";
import { Image, View } from "@tarojs/components";

import { designPxToRpx, formatFrame } from "@/components/modules";
import CommonTitle from "@/pages/customPage/modules/commonTitle/index";
import { getHomeSeckill } from "@/api/home/home";
import { useConfigStore } from "@/store/config";
import { resolveH5BuyBtnStyle } from "@/utils/buyStyle";
import { redirect } from "@/utils";
import { isShowB2bPriceText } from "@/utils/b2bPrice";
import { useProductStyle } from "@/hooks/useProductStyle";
import TigImage from "@/components/tig/tig-image/TigImage";
import FormatPrice from "@/components/format-price/FormatPrice";
import { t } from "@/utils/i18n";

import "./index.scss";

interface Props {
    module: any;
    moduleIndex?: number;
    index?: number;
}

export default function Seckill2({ module }: Props) {
    const { frame } = module;

    // 购买按钮「跟随全局」解析（读时、不回写）：buyBtnStyle=-1 时取全局样式
    // 降级到 0–8；其余值原样透传，渲染与此前逐字一致。本模块无独立按钮
    // 宿主元素（按钮由根类名驱动 CSS），仅解析类名值。
    const globalBuyStyle = useConfigStore((s) => s.buyStyle);
    const buyBtn = resolveH5BuyBtnStyle(module?.buyBtnStyle, globalBuyStyle);
    const resolvedBuyBtnStyle = buyBtn.style ?? module?.buyBtnStyle;

    const frameFormat = useMemo(() => {
        return formatFrame(frame ?? {});
    }, [frame]);

    // 旧 computed 一次性读取（非响应式依赖），React 侧挂载时求值一次
    const showB2bText = useMemo(() => isShowB2bPriceText(), []);

    const allFormat = useMemo(() => {
        return {
            backgroundColor: `background-color: ${module.backgroundColor};`,
            boxPadding: `margin-left: ${designPxToRpx(module.boxPadding)};margin-right: ${designPxToRpx(module.boxPadding)};`,
            boxPaddingBottom: `margin-bottom: ${designPxToRpx(module.boxPaddingBottom)};`,
            boxPaddingTop: `margin-top: ${designPxToRpx(module.boxPaddingTop)};`,
            boxRadius: `border-radius: ${designPxToRpx(module.boxRadius)};`,
            innerPadding: `padding: ${designPxToRpx(module.innerPadding)};`
        };
    }, [module]);

    const itemContentStyle = useMemo(() => {
        return `${module.style == 10 ? "padding-left" : "padding"}: ${designPxToRpx(module.goodsPadding)};`;
    }, [module.style, module.goodsPadding]);

    const goodsConStyle = useMemo(() => {
        let str = "";

        if (module.style == 10) {
            str += `display: flex; overflow: hidden;overflow-x: auto;flex-wrap: nowrap;padding-bottom: 20rpx;`;
        }

        return str;
    }, [module.style]);

    const adItemStyle = useMemo(() => {
        return module.style == 10 ? `width: ${100 / 3}%;flex-shrink: 0;flex-grow: 0;` : "";
    }, [module.style]);

    const getDiscount = (currentPrice: number, originalPrice: number) => {
        if (originalPrice === 0 || currentPrice >= originalPrice) {
            return 0;
        }

        const discountPercentage = ((originalPrice - currentPrice) / originalPrice) * 100;

        return Math.floor(discountPercentage * 10) / 10;
    };

    const [seckillList, setSeckillList] = useState<any[]>([]);
    const { styleMap } = useProductStyle(seckillList);
    const getData = async () => {
        try {
            const result = await getHomeSeckill();
            setSeckillList(result.records);
        } catch (error) {
            console.error(error);
        }
    };

    // 对应旧 onMounted
    useEffect(() => {
        void getData();
        // eslint-disable-next-line react-hooks/exhaustive-deps
    }, []);

    return (
        seckillList && seckillList?.length > 0 && (
            <View
                className={
                    "module-ad-con decorate-seckill2 module-goods_ad module-seckill-goods_ad ad-style__" +
                    module?.style +
                    " ad-buy_btn_style__" +
                    resolvedBuyBtnStyle +
                    " ad-goods_style__" +
                    module?.goodsStyle +
                    " ad-goods_radio_style__" +
                    module?.goodsRadioStyle +
                    " ad-text_align__" +
                    module?.textAlign +
                    " ad-text_weight__" +
                    module?.textWeight +
                    " ad-goods_name_row__" +
                    module?.goodsNameRow +
                    " ad-goods_name_padding__" +
                    module?.goodsNamePadding
                }
                style={frameFormat.boxPadding + frameFormat.boxPaddingTop + frameFormat.boxPaddingBottom + allFormat.backgroundColor}
            >
                <View
                    className="module-ad-content"
                    style={
                        frameFormat.backgroundColor +
                        " " +
                        frameFormat.boxRadius +
                        allFormat.boxPadding +
                        allFormat.boxPaddingBottom +
                        allFormat.boxPaddingTop +
                        allFormat.boxRadius +
                        allFormat.innerPadding
                    }
                >
                    {/* 原版 seckill2 曾将「距结束」倒计时通过 #more 插槽传入 commonTitle，但原版 commonTitle 无插槽出口、从未渲染；
                        P1 对齐：不再传入，标题栏右侧与原版一致（仅标题/默认「查看更多」）。秒杀结束时间等数据字段保持不动。 */}
                    <CommonTitle module={module.title} />

                    <View className="goods-ad-warp">
                        <View className="goods-ad-con" style={goodsConStyle}>
                            {seckillList.map((item: any) => (
                                <View key={item.productId} className="goods-ad-item" style={adItemStyle}>
                                    <View className="item-content" style={itemContentStyle}>
                                        {/* 旧 navigator :url → View onClick + redirect */}
                                        <View
                                            className="item-image-a"
                                            onClick={() => redirect({ url: "/pages/product/index?id=" + item.productId })}
                                        >
                                            <View className="item-con">
                                                <View className="item-photo">
                                                    <TigImage src={item.picUrl} mode="widthFix" />

                                                    {styleMap[item.productId]?.borderImage && (
                                                        <Image
                                                            className="product-style-border"
                                                            src={styleMap[item.productId].borderImage}
                                                            mode="scaleToFill"
                                                        />
                                                    )}

                                                    <View className="item-photo-bg" style={{ backgroundColor: module.imageBackgroundColor }} />
                                                </View>
                                                <View className="item-info">
                                                    {module.showName == 1 && (
                                                        <View className="item-name">
                                                            <View
                                                                className="item-name-a"
                                                                onClick={() => redirect({ url: "/pages/product/index?id=" + item.productId })}
                                                            >
                                                                {item.productName ?? ""}
                                                            </View>
                                                        </View>
                                                    )}
                                                    <View className="tips">
                                                        {t("优惠{0}%", {
                                                            0: getDiscount(Number(item.seckillPrice), Number(item.marketPrice))
                                                        })}
                                                    </View>
                                                    {module.showPrice ? (
                                                        <View className="item-action">
                                                            <View className="item-price">
                                                                <FormatPrice displayPrice
                                                                    fontStyle={{ fontSize: "32rpx", fontWeight: "bold" }}
                                                                    currencyStyle={{
                                                                        fontSize: "24rpx",
                                                                        fontWeight: "bold"
                                                                    }}
                                                                    decimalsStyle={{
                                                                        fontSize: "24rpx",
                                                                        fontWeight: "bold"
                                                                    }}
                                                                    priceData={item.seckillPrice}
                                                                />
                                                                {module.style == 10 && !showB2bText && (
                                                                    <View className="price-through">
                                                                        <FormatPrice displayPrice
                                                                            fontStyle={{ textDecoration: "line-through" }}
                                                                            priceData={item.orgProductPrice}
                                                                            showDecimals={false}
                                                                        />
                                                                    </View>
                                                                )}
                                                            </View>
                                                        </View>
                                                    ) : null}
                                                </View>
                                            </View>
                                        </View>
                                    </View>
                                </View>
                            ))}
                        </View>
                    </View>
                </View>
            </View>
        )
    );
}
