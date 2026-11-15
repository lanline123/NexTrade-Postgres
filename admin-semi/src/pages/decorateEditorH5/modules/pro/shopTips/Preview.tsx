/**
 * 商品提示 预览（移植自 view/admin editor/modules/mobilePro/shopTips/Preview.vue）
 *
 * 按 rowNum 分行（flex space-around），每项图标 14×14 + 文案 12px，间隔由 itemGap 半距驱动，
 * showDivider=1 时非末项右侧出竖线（颜色同 titleColor）；空态为居中提示文案。
 */
import { useMemo } from "react";
import { cssText, productImage } from "../../../shared/format";
import { mergeDefaultModule, formatCommonStyle } from "../../../contract/sharedDefaults";
import { shopTipsDefaults } from "./index";
import "./Preview.css";

export default function ShopTipsPreview({ data }: { data: Record<string, any> }) {
    const module = useMemo(() => {
        const merged = JSON.parse(JSON.stringify(data ?? {}));
        mergeDefaultModule(merged, shopTipsDefaults);
        return merged;
    }, [data]);

    const commonStyle = formatCommonStyle(module.moduleStyle ?? {}, module.contentStyle ?? {});
    const picList: any[] = Array.isArray(module.picList) ? module.picList : [];
    const rowNum = Number(module.rowNum) || 3;
    const halfGap = (Number(module.itemGap) || 0) / 2;
    const rows: any[][] = [];
    for (let i = 0; i < picList.length; i += rowNum) rows.push(picList.slice(i, i + rowNum));
    const textColor = module.titleColor || "#333333";

    return (
        <div
            className="module-ad-con module-shop-tips h5-shop-tips"
            style={cssText(
                commonStyle.moduleStyle.boxPaddingTop,
                commonStyle.moduleStyle.boxPaddingBottom,
                commonStyle.moduleStyle.boxPadding,
                commonStyle.moduleStyle.boxMargin,
                commonStyle.moduleStyle.backgroundSize,
                commonStyle.moduleStyle.backgroundImage
            )}
        >
            <div
                className="module-ad-content"
                style={cssText(
                    commonStyle.moduleContentStyle.backgroundImage,
                    commonStyle.moduleContentStyle.backgroundSize,
                    commonStyle.moduleContentStyle.boxRadius2,
                    commonStyle.moduleContentStyle.boxShadow,
                    commonStyle.moduleContentStyle.innerPadding
                )}
            >
                {picList.length === 0 ? (
                    <div className="shop-tips-empty">
                        <div className="desc">点击添加商品提示</div>
                    </div>
                ) : (
                    <div className="shop-tips-outer">
                        {rows.map((row, rowIndex) => (
                            <ul
                                className="shop-tips-row"
                                key={rowIndex}
                                style={halfGap > 0 ? { marginLeft: -halfGap, marginRight: -halfGap } : undefined}
                            >
                                {row.map((item, colIndex) => (
                                    <li className="shop-tips-li" key={colIndex}>
                                        <a
                                            className="shop-tips-link"
                                            style={{
                                                ...(halfGap > 0 ? { margin: halfGap } : {}),
                                                ...(module.itemBackgroundColor ? { backgroundColor: module.itemBackgroundColor } : {})
                                            }}
                                        >
                                            <div className="shop-tips-content" style={{ color: textColor }}>
                                                {item.picUrl || item.picThumb ? <img className="shop-tips-icon" src={productImage(item)} alt="" /> : null}
                                                <span className="shop-tips-text">{item.picTitle}</span>
                                            </div>
                                            {Number(module.showDivider) === 1 && colIndex < row.length - 1 ? (
                                                <em className="shop-tips-sx" style={{ borderColor: textColor }} />
                                            ) : null}
                                        </a>
                                    </li>
                                ))}
                            </ul>
                        ))}
                    </div>
                )}
            </div>
        </div>
    );
}
