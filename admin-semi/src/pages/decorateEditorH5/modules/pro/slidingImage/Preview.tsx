/**
 * 图片滑动 预览（类名对齐 uniapp-react slidingImage：slidingImage/slidingImage-box/item/item-box/title/desc）
 */
import { useMemo } from "react";
import { cssText, imageFormat } from "../../../shared/format";
import { mergeDefaultModule, formatCommonStyle } from "../../../contract/sharedDefaults";
import { slidingImageDefaults } from "./index";
import "./Preview.css";

export default function SlidingImagePreview({ data }: { data: Record<string, any> }) {
    const module = useMemo(() => {
        const merged = JSON.parse(JSON.stringify(data ?? {}));
        mergeDefaultModule(merged, slidingImageDefaults);
        return merged;
    }, [data]);
    const commonStyle = formatCommonStyle(module.moduleStyle ?? {}, module.contentStyle ?? {}, module.subTitle ?? {});
    const picList: any[] = module.picList || [];
    return (
        <div className="module-ad-con" style={cssText(commonStyle.moduleStyle.boxPaddingBottom, commonStyle.moduleStyle.boxPaddingTop, commonStyle.moduleStyle.boxPadding, commonStyle.moduleStyle.backgroundSize, commonStyle.moduleStyle.backgroundImage)}>
            <div className="module-ad-content" style={cssText(commonStyle.moduleContentStyle.backgroundImage, commonStyle.moduleContentStyle.backgroundSize, commonStyle.moduleContentStyle.boxRadius, commonStyle.moduleContentStyle.boxShadow, commonStyle.moduleContentStyle.innerPadding)}>
                <div className="slidingImage">
                    {picList.length === 0 ? (<div className="module-ad-empty empty-image_ad"><div className="image-empty-bg" /><div className="desc">点击添加图片</div></div>) : (
                        <div className="slidingImage-box" style={{ display: "flex", gap: `${module.picMargin ?? 8}px`, overflowX: "auto", padding: `${module.picPadding ?? 0}px` }}>
                            {picList.map((item, i) => (
                                <div className="item" key={i} style={{ flex: `0 0 ${module.picWidth ?? 80}%` }}>
                                    <div className="item-box">
                                        <img className="img" src={imageFormat(item.picUrl)} alt="" style={{ borderRadius: `${module.picRadius ?? 8}px`, objectFit: module.picFillType ?? "cover", width: "100%", display: "block" }} />
                                        {item.title ? <div className="title line2 fox-title" style={{ fontSize: `${module.subTitle?.titleFontSize ?? 13}px`, color: module.subTitle?.titleColor, fontWeight: module.subTitle?.titleFontBold }}>{item.title}</div> : null}
                                        {Number(module.showDesc) === 1 && item.desc ? <div className="desc line1" style={{ color: module.descColor }}>{item.desc}</div> : null}
                                    </div>
                                </div>
                            ))}
                        </div>
                    )}
                </div>
            </div>
        </div>
    );
}
