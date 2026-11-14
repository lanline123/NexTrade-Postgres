/**
 * 分类导航A1 预览（类名对齐 uniapp-react categoryNavA1：category-nav/category-nav-item/item-box/scroll-box）
 */
import { useMemo } from "react";
import { cssText, imageFormat } from "../../../shared/format";
import { mergeDefaultModule, formatCommonStyle } from "../../../contract/sharedDefaults";
import { categoryNavA1Defaults } from "./index";
import "./Preview.css";

export default function categoryNavA1Preview({ data }: { data: Record<string, any> }) {
    const module = useMemo(() => {
        const merged = JSON.parse(JSON.stringify(data ?? {}));
        mergeDefaultModule(merged, categoryNavA1Defaults);
        return merged;
    }, [data]);
    const commonStyle = formatCommonStyle(module.moduleStyle ?? {}, module.contentStyle ?? {}, module.subTitle ?? {});
    const categoryList: any[] = module.categoryList || [];
    const subTitle = commonStyle.moduleSubTitle || {};
    return (
        <div className="module-ad-con" style={cssText(commonStyle.moduleStyle.boxPaddingBottom, commonStyle.moduleStyle.boxPaddingTop, commonStyle.moduleStyle.boxPadding, commonStyle.moduleStyle.backgroundSize, commonStyle.moduleStyle.backgroundImage)}>
            <div className="module-ad-content" style={cssText(commonStyle.moduleContentStyle.backgroundImage, commonStyle.moduleContentStyle.backgroundSize, commonStyle.moduleContentStyle.boxRadius, commonStyle.moduleContentStyle.boxShadow, commonStyle.moduleContentStyle.innerPadding)}>
                {module.logoPic?.picUrl ? <img className="catnav-a3-logo" src={imageFormat(module.logoPic.picUrl)} alt="" /> : null}
                <div className="category-nav">
                    <div className="scroll-box" style={{ overflowX: module.listStyleType === "slide" ? "auto" : "visible" }}>
                        <div className={`decorate-slide-box ${module.listStyleType}`} style={{ display: "flex", flexWrap: module.listStyleType === "slide" ? "nowrap" : "wrap" }}>
                            {categoryList.map((item, index) => (
                                <div className="category-nav-item" key={index} style={{ flex: `0 0 ${100 / (module.rowNum || 5)}%` }}>
                                    <div className="item-box" style={{ padding: `${module.categoryPadding}px` }}>
                                        <div className="category-nav-item-img" style={{ paddingTop: `${module.picPercent}%`, width: `${module.picPercent}%`, borderRadius: `${module.radius}px`, backgroundSize: module.picFillType, backgroundImage: `url(${imageFormat(item.picUrl)})` }} />
                                        {module.subTitle?.showTitle ? <p className="category-nav-item-text line1" style={cssText(subTitle.textBlod, subTitle.textColor, subTitle.textFontSize)}>{item.picTitle}</p> : null}
                                        
                                    </div>
                                </div>
                            ))}
                        </div>
                    </div>
                    
                </div>
            </div>
        </div>
    );
}
