/**
 * 大图滑动 预览（移植自 view/admin editor/modules/mobilePro/slideLargeImage/Preview.vue）
 *
 * 原版：列表复制 5 份、每项宽 = 容器宽/rowNum，图高按首图天然宽高比（padding-top 比值），
 * auto 时 .slidelarge-image-box 以 left 关键帧 5000ms 线性无限滚动。
 * 编辑器承接：横向 flex 行（每项 flex-basis 100/rowNum %），图以 <img> 天然比例呈现
 * （等价于原版按天然比撑高），auto 时保留原版 left 动画类与时长口径（animation-duration=slidingSpeed s）。
 */
import { useMemo } from "react";
import { cssText, imageFormat } from "../../../shared/format";
import { mergeDefaultModule, formatCommonStyle, copyArray } from "../../../contract/sharedDefaults";
import { slideLargeImageDefaults } from "./index";
import "./Preview.css";

export default function SlideLargeImagePreview({ data }: { data: Record<string, any> }) {
    const module = useMemo(() => {
        const merged = JSON.parse(JSON.stringify(data ?? {}));
        mergeDefaultModule(merged, slideLargeImageDefaults);
        return merged;
    }, [data]);

    const commonStyle = formatCommonStyle(module.moduleStyle ?? {}, module.contentStyle ?? {}, module.subTitle ?? {});
    const picList: any[] = Array.isArray(module.picList) ? module.picList : [];
    const list = copyArray(picList, 5) as any[];
    const titleDesc = module.picTitleDesc ?? {};
    const modTitle = module.moduleTitle ?? {};
    const rowNum = Number(module.rowNum) || 1.5;

    const renderTitle = () => (
        <div
            style={{
                textAlign: modTitle.textAlignment,
                margin: `${modTitle.titleMarginTop}px ${modTitle.titleMarginLeft}px ${modTitle.titleMarginBottom}px`
            }}
        >
            <div style={{ fontSize: `${modTitle.titleFontSize}px`, fontWeight: modTitle.titleFontWeight, color: modTitle.titleColor }}>{modTitle.titleText}</div>
            <div style={{ fontSize: `${modTitle.subFontTitleSize}px`, marginTop: 8, fontWeight: modTitle.subFontTitleWeight, color: modTitle.subFontTitleColor }}>
                {modTitle.subTitleText}
            </div>
        </div>
    );

    return (
        <div
            className="module-ad-con h5-slide-large"
            style={cssText(
                commonStyle.moduleStyle.boxPaddingBottom,
                commonStyle.moduleStyle.boxPaddingTop,
                commonStyle.moduleStyle.boxPadding,
                commonStyle.moduleStyle.backgroundSize,
                commonStyle.moduleStyle.backgroundImage
            )}
        >
            <div
                className="module-ad-content"
                style={cssText(
                    commonStyle.moduleContentStyle.backgroundImage,
                    commonStyle.moduleContentStyle.backgroundSize,
                    commonStyle.moduleContentStyle.boxRadius,
                    commonStyle.moduleContentStyle.boxShadow,
                    commonStyle.moduleContentStyle.innerPadding
                )}
            >
                <div className="slidelarge-image">
                    {modTitle && modTitle.titlePosition === "top" && modTitle.titleText ? renderTitle() : null}
                    <div style={{ marginTop: `${module.picMarginTop}%`, width: "100%", zIndex: 1, overflow: "hidden" }}>
                        <div
                            className={module.slidingEffect === "auto" ? "slidelarge-image-box" : undefined}
                            style={{
                                display: "flex",
                                animationDuration: `${module.slidingSpeed}s`,
                                position: "relative",
                                top: 0,
                                left: 0,
                                animationPlayState: module.slidingEffect === "manual" ? "paused" : "running"
                            }}
                        >
                            {list.map((item, index) => (
                                <div key={index} style={{ flexBasis: `${100 / rowNum}%`, flexShrink: 0 }}>
                                    <div style={{ marginRight: `${module.picMargin}px`, background: "none", borderRadius: `${module.picRadius}px`, position: "relative", overflow: "hidden" }}>
                                        <img
                                            src={imageFormat(item.picUrl)}
                                            alt=""
                                            style={{ width: "100%", display: "block", objectFit: module.picFillType === "contain" ? "contain" : "cover" }}
                                        />
                                        {titleDesc.show && (item.picTitle || item.picDesc) ? (
                                            <div
                                                style={{
                                                    background: titleDesc.backgroundColor ? titleDesc.backgroundColor : "none",
                                                    textAlign: titleDesc.textAlignment,
                                                    ...(titleDesc.fontPosition === "internal"
                                                        ? { position: "absolute" as const, left: 0, right: 0, bottom: 0, padding: "10px 15px" }
                                                        : { margin: `${titleDesc.marginTop}px ${titleDesc.marginLeft}px ${titleDesc.marginBottom}px` })
                                                }}
                                            >
                                                {item.picTitle ? (
                                                    <div style={{ color: titleDesc.titleColor, fontSize: `${titleDesc.titleFontSize}px`, fontWeight: titleDesc.titleFontWeight, overflow: "hidden" }}>
                                                        {item.picTitle}
                                                    </div>
                                                ) : null}
                                                {item.picDesc ? (
                                                    <div style={{ marginTop: 6, fontSize: `${titleDesc.descFontSize}px`, fontWeight: titleDesc.descFontWeight || undefined, color: titleDesc.descColor, overflow: "hidden" }}>
                                                        {item.picDesc}
                                                    </div>
                                                ) : null}
                                            </div>
                                        ) : null}
                                    </div>
                                </div>
                            ))}
                        </div>
                    </div>
                    {modTitle && modTitle.titlePosition === "bottom" ? renderTitle() : null}
                </div>
            </div>
        </div>
    );
}
