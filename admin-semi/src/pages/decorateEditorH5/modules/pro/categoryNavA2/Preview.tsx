/**
 * 类目导航A2 预览（移植自 view/admin editor/modules/mobilePro/categoryNavA2/Preview.vue）
 *
 * 与 A1 差异：无图背色层；非 list 态底部有指示条（barWidth/barHeight/barWidthOn/barColor/barColorOn）。
 * 原版以容器实测宽度算每项 px 宽（rowNum="custom" 时按 rowPicPercent%）；编辑器内以 flex-basis 百分比等价承接。
 */
import { useMemo } from "react";
import { cssText, imageFormat } from "../../../shared/format";
import { mergeDefaultModule, formatCommonStyle } from "../../../contract/sharedDefaults";
import { categoryNavA2Defaults } from "./index";
import "./Preview.css";

export default function CategoryNavA2Preview({ data }: { data: Record<string, any> }) {
    const module = useMemo(() => {
        const merged = JSON.parse(JSON.stringify(data ?? {}));
        mergeDefaultModule(merged, categoryNavA2Defaults);
        return merged;
    }, [data]);

    const commonStyle = formatCommonStyle(module.moduleStyle ?? {}, module.contentStyle ?? {}, module.subTitle ?? {});
    const categoryList: any[] = module.categoryList || [];
    const basis = module.rowNum === "custom" ? Number(module.rowPicPercent) || 18 : 100 / (Number(module.rowNum) || 5);

    return (
        <div
            className="module-ad-con h5-catnav-a2"
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
                    commonStyle.moduleContentStyle.innerPadding,
                    commonStyle.moduleContentStyle.paddingBottom,
                    commonStyle.moduleContentStyle.paddingTop,
                    commonStyle.moduleContentStyle.paddingLeftRight
                )}
            >
                {categoryList.length === 0 ? (
                    <div className="module-ad-empty empty-image_ad">
                        <div className="image-empty-bg" />
                        <div className="desc">点击添加分类</div>
                    </div>
                ) : (
                    <>
                        <div className="category-nav">
                            <div className={`decorate-slide-box ${module.listStyleType}`}>
                        {categoryList.map((item, index) => (
                            <div
                                className="category-nav-item"
                                key={index}
                                style={{
                                    transform: "translate(0px, 0px) scale(1) rotateY(0deg)",
                                    opacity: 1,
                                    transitionDuration: "0s",
                                    flexShrink: 0,
                                    flexBasis: `${basis}%`
                                }}
                            >
                                <div className="item-box" style={{ padding: `${module.categoryPadding}px` }}>
                                    <div
                                        className="category-nav-item-img"
                                        style={{
                                            backgroundImage: `url(${imageFormat(item.picUrl)})`,
                                            paddingTop: `${module.picPercent}%`,
                                            width: `${module.picPercent}%`,
                                            borderRadius: `${module.radius}px`,
                                            backgroundSize: module.picFillType
                                        }}
                                    />
                                    {module.subTitle?.showTitle ? (
                                        <p
                                            className="category-nav-item-text"
                                            style={cssText(commonStyle.moduleSubTitle.textBlod, commonStyle.moduleSubTitle.textColor, commonStyle.moduleSubTitle.textFontSize)}
                                        >
                                            {item.picTitle}
                                        </p>
                                    ) : null}
                                </div>
                            </div>
                        ))}
                    </div>
                </div>
                {module.listStyleType !== "list" && Number(module.indicatorBar) === 1 ? (
                    <div className="line-box">
                        <div
                            className="line"
                            style={{ margin: "0 auto", width: `${module.barWidth}%`, height: `${module.barHeight}px`, borderRadius: 4, background: module.barColor, overflow: "hidden" }}
                        >
                            <div style={{ width: `${module.barWidthOn}%`, height: `${module.barHeight}px`, borderRadius: 10, background: module.barColorOn, transition: "0.3s", position: "relative", left: "0%" }} />
                        </div>
                    </div>
                ) : null}
                    </>
                )}
            </div>
        </div>
    );
}
