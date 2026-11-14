/**
 * 分类推荐A1 预览（类名对齐 uniapp-react categoryRecA1）
 */
import { useMemo } from "react";
import { cssText, imageFormat } from "../../../shared/format";
import { mergeDefaultModule, formatCommonStyle } from "../../../contract/sharedDefaults";
import { categoryRecA1Defaults } from "./index";
import "./Preview.css";

export default function CategoryRecA1Preview({ data }: { data: Record<string, any> }) {
    const module = useMemo(() => {
        const merged = JSON.parse(JSON.stringify(data ?? {}));
        mergeDefaultModule(merged, categoryRecA1Defaults);
        return merged;
    }, [data]);
    const commonStyle = formatCommonStyle(module.moduleStyle ?? {}, module.contentStyle ?? {}, module.subTitle ?? {});
    const picList: any[] = module.picList || [];
    return (
        <div className="module-ad-con module-category-rec-a1" style={cssText(commonStyle.moduleStyle.boxPaddingBottom, commonStyle.moduleStyle.boxPaddingTop, commonStyle.moduleStyle.boxPadding, commonStyle.moduleStyle.backgroundSize, commonStyle.moduleStyle.backgroundImage)}>
            <div className="module-ad-content" style={cssText(commonStyle.moduleContentStyle.backgroundImage, commonStyle.moduleContentStyle.backgroundSize, commonStyle.moduleContentStyle.boxRadius, commonStyle.moduleContentStyle.boxShadow, commonStyle.moduleContentStyle.innerPadding)}>
                {picList.length === 0 ? (<div className="module-ad-empty empty-image_ad"><div className="image-empty-bg" /><div className="desc">点击添加图片</div></div>) : (
                    <div className="category-rec-a1-list">
                        {picList.map((item, index) => (
                            <div className="category-rec-a1-item" key={index} style={{ borderRadius: `${module.picRadius ?? 8}px` }}>
                                <img className="item-bg" src={imageFormat(item.picUrl)} alt="" style={{ objectFit: module.picFillType ?? "cover" }} />
                                <div className="item-inner"><div className="item-content">
                                    <div className="item-title" style={{ fontSize: `${module.subTitle?.titleFontSize ?? 14}px`, fontWeight: module.subTitle?.titleFontBold, color: module.subTitle?.titleColor }}>{item.title}</div>
                                    <div className="item-desc" style={{ color: module.subTitleColor }}>{item.desc}</div>
                                </div></div>
                            </div>
                        ))}
                    </div>
                )}
            </div>
        </div>
    );
}
