/**
 * 店铺提示 预览（类名对齐 uniapp-react shopTips：shop-tips-outer/row/li/icon/text/sx）
 */
import { useMemo } from "react";
import { cssText, imageFormat, pickPic } from "../../../shared/format";
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
    const picList: any[] = module.picList || [];
    return (
        <div className="module-ad-con module-shop-tips" style={cssText(commonStyle.moduleStyle.boxPaddingBottom, commonStyle.moduleStyle.boxPaddingTop, commonStyle.moduleStyle.boxPadding, commonStyle.moduleStyle.backgroundSize, commonStyle.moduleStyle.backgroundImage)}>
            <div className="module-ad-content" style={cssText(commonStyle.moduleContentStyle.backgroundImage, commonStyle.moduleContentStyle.backgroundSize, commonStyle.moduleContentStyle.boxRadius, commonStyle.moduleContentStyle.boxShadow, commonStyle.moduleContentStyle.innerPadding)}>
                <div className="shop-tips-outer" style={{ backgroundColor: module.itemBackgroundColor || undefined }}>
                    <div className="shop-tips-row" style={{ gap: `${module.itemGap ?? 8}px`, display: "grid", gridTemplateColumns: `repeat(${module.rowNum || 3}, 1fr)` }}>
                        {picList.map((item, i) => (
                            <div className="shop-tips-li" key={i}>
                                <div className="shop-tips-content">
                                    {pickPic(item) ? <img className="shop-tips-icon" src={imageFormat(pickPic(item))} alt="" /> : <span className="shop-tips-icon shop-tips-sx">✓</span>}
                                    <span className="shop-tips-text" style={{ color: module.titleColor }}>{item.title}</span>
                                    {Number(module.showDivider) === 1 && i < picList.length - 1 ? <span className="shop-tips-link" /> : null}
                                </div>
                            </div>
                        ))}
                    </div>
                </div>
            </div>
        </div>
    );
}
