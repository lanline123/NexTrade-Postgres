/**
 * 店铺导航 预览（渲染端暂无入口，此预览按店铺导航通用结构还原，供存量数据在编辑器内查看）
 */
import { useMemo } from "react";
import { cssText, imageFormat } from "../../../shared/format";
import { mergeDefaultModule, formatCommonStyle } from "../../../contract/sharedDefaults";
import { storeNavDefaults } from "./index";
import "./Preview.css";

export default function StoreNavPreview({ data }: { data: Record<string, any> }) {
    const module = useMemo(() => {
        const merged = JSON.parse(JSON.stringify(data ?? {}));
        mergeDefaultModule(merged, storeNavDefaults);
        return merged;
    }, [data]);
    const commonStyle = formatCommonStyle(module.moduleStyle ?? {}, module.contentStyle ?? {}, module.subTitle ?? {});
    const picList: any[] = module.picList || [];
    const subTitle = commonStyle.moduleSubTitle || {};
    return (
        <div className="module-ad-con" style={cssText(commonStyle.moduleStyle.boxPaddingBottom, commonStyle.moduleStyle.boxPaddingTop, commonStyle.moduleStyle.boxPadding, commonStyle.moduleStyle.backgroundSize, commonStyle.moduleStyle.backgroundImage)}>
            <div className="module-ad-content" style={cssText(commonStyle.moduleContentStyle.backgroundImage, commonStyle.moduleContentStyle.backgroundSize, commonStyle.moduleContentStyle.boxRadius, commonStyle.moduleContentStyle.boxShadow, commonStyle.moduleContentStyle.innerPadding)}>
                <div className="store-nav">
                    {picList.map((item, index) => (
                        <div className="store-nav-item" key={index} style={{ flex: `0 0 ${100 / (module.rowNum || 4)}%` }}>
                            <div className="item-box" style={{ padding: `${module.categoryPadding ?? 8}px` }}>
                                <div className="store-nav-item-img" style={{ paddingTop: `${module.picPercent ?? 80}%`, width: `${module.picPercent ?? 80}%`, borderRadius: `${module.radius ?? 200}px`, backgroundSize: module.picFillType, backgroundImage: `url(${imageFormat(item.picUrl)})` }} />
                                {module.subTitle?.showTitle ? <p className="store-nav-item-text" style={cssText(subTitle.textBlod, subTitle.textColor, subTitle.textFontSize)}>{item.picTitle || item.title}</p> : null}
                            </div>
                        </div>
                    ))}
                </div>
            </div>
        </div>
    );
}
