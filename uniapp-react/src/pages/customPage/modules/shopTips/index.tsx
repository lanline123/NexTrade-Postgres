import { useMemo } from "react";
import { Image, Text, View } from "@tarojs/components";

import { designPxToRpx, formatCommonStyle } from "@/components/modules";
import { redirect } from "@/utils";
import { productImage, urlFormat } from "@/utils/format";
import { t } from "@/utils/i18n";

import "./index.scss";

interface Props {
    module: any;
    moduleIndex?: number;
    index?: number;
}

export default function ShopTips({ module }: Props) {
    const commonStyle = useMemo(() => formatCommonStyle(module.moduleStyle ?? {}, module.contentStyle ?? {}), [module]);

    const rowNum = useMemo(() => Number(module.rowNum) || 3, [module.rowNum]);
    const halfGap = useMemo(() => (module.itemGap ?? 0) / 2, [module.itemGap]);

    const itemRows = useMemo(() => {
        const list = module.picList || [];
        const n = rowNum;
        const rows: any[][] = [];
        for (let i = 0; i < list.length; i += n) {
            rows.push(list.slice(i, i + n));
        }
        return rows;
    }, [module.picList, rowNum]);

    const rowStyle = useMemo(() => {
        const h = halfGap;
        return h > 0 ? `margin-left: ${designPxToRpx(-h)}; margin-right: ${designPxToRpx(-h)};` : "";
    }, [halfGap]);

    const linkStyle = useMemo(() => {
        const h = halfGap;
        const bgColor = module.itemBackgroundColor;
        const bgStyle = bgColor ? `background-color: ${bgColor};` : "";
        const marginStyle = h > 0 ? `margin: ${designPxToRpx(h)};` : "";
        return marginStyle + bgStyle;
    }, [halfGap, module.itemBackgroundColor]);

    // 条目级标题颜色（我方新增 2026-11-21）：item.titleColor 优先，未设回退模块级
    // titleColor（原版口径，快照仓 v5.8.61 同路径渲染器只有模块级一份）；竖线仍走模块级色
    const itemTextStyle = (item: any) => `color: ${item.titleColor || module.titleColor || "#333333"};`;

    const dividerStyle = useMemo(() => {
        const color = module.titleColor || "#333333";
        return `border-color: ${color};`;
    }, [module.titleColor]);

    return (
        <View
            className="module-ad-con module-shop-tips"
            style={
                commonStyle.moduleStyle.boxPaddingTop +
                commonStyle.moduleStyle.boxPaddingBottom +
                commonStyle.moduleStyle.boxPadding +
                commonStyle.moduleStyle.boxMargin +
                commonStyle.moduleStyle.backgroundSize +
                commonStyle.moduleStyle.backgroundImage
            }
        >
            <View
                className="module-ad-content"
                style={
                    commonStyle.moduleContentStyle.backgroundImage +
                    commonStyle.moduleContentStyle.backgroundSize +
                    commonStyle.moduleContentStyle.boxRadius2 +
                    commonStyle.moduleContentStyle.boxShadow +
                    commonStyle.moduleContentStyle.innerPadding
                }
            >
                {module.picList && module.picList.length > 0 && (
                    <View className="shop-tips-outer">
                        {itemRows.map((row: any[], rowIndex: number) => (
                            <View className="shop-tips-row" key={rowIndex} style={rowStyle}>
                                {row.map((item: any, colIndex: number) => (
                                    <View className="shop-tips-li" key={colIndex}>
                                        <View
                                            className="shop-tips-link"
                                            style={linkStyle}
                                            onClick={() => {
                                                urlFormat(item.picLink) && redirect({ url: urlFormat(item.picLink) });
                                            }}
                                        >
                                            <View className="shop-tips-content" style={itemTextStyle(item)}>
                                                {(item.picUrl || item.picThumb) && (
                                                    <Image className="shop-tips-icon" src={productImage(item)} mode="aspectFit" />
                                                )}
                                                <Text className="shop-tips-text">{t(item.picTitle || "")}</Text>
                                            </View>
                                            {Number(module.showDivider) === 1 && colIndex < row.length - 1 && (
                                                <View className="shop-tips-sx" style={dividerStyle} />
                                            )}
                                        </View>
                                    </View>
                                ))}
                            </View>
                        ))}
                    </View>
                )}
            </View>
        </View>
    );
}
