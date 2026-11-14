/**
 * 分类导航A3 预览（类名对齐 uniapp-react categoryNavA3：category-nav/category-nav-item/item-box/scroll-box）
 */
import { useMemo } from "react";
import { cssText, imageFormat } from "../../../shared/format";
import { mergeDefaultModule, formatCommonStyle } from "../../../contract/sharedDefaults";
import { categoryNavA3Defaults } from "./index";
import "./Preview.css";

export default function categoryNavA3Preview({ data }: { data: Record<string, any> }) {
    const module = useMemo(() => {
        const merged = JSON.parse(JSON.stringify(data ?? {}));
        mergeDefaultModule(merged, categoryNavA3Defaults);
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
                                        {item.picDesc ? (<div className="category-nav-item-desc"><span className="desc-text">{item.picDesc}</span></div>) : null}
                                    </div>
                                </div>
                            ))}
                        </div>
                    </div>
                    {Number(module.indicatorBar) === 1 ? (<div className="line-box"><div className="line" style={{ width: `${module.barWidthOn ?? 30}px`, height: `${module.barHeight ?? 3}px`, backgroundColor: module.barColorOn }} /><div className="line" style={{ width: `${module.barWidth ?? 20}px`, height: `${module.barHeight ?? 3}px`, backgroundColor: module.barColor }} /></div>) : null}
                </div>
            </div>
        </div>
    );
}
