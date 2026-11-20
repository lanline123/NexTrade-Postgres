/**
 * 类目导航A1 预览（移植自 view/admin editor/modules/mobilePro/categoryNavA1/Preview.vue）
 *
 * 与 categoryNav 的差异：图盒为相对定位容器（imageBackgroundColor 底层 + 图绝对填充），
 * 原版以容器实测宽度计算每项 px 宽；编辑器内以 flex-basis 百分比等价承接（100/rowNum %）。
 * showSplitLine / radiusStyle 在原版 Preview 模板中未被消费（仅 Settings 可编辑），此处同样不消费。
 */
import { useMemo } from "react";
import { cssText, imageFormat } from "../../../shared/format";
import { mergeDefaultModule, formatCommonStyle } from "../../../contract/sharedDefaults";
import { categoryNavA1Defaults } from "./index";
import "./Preview.css";

export default function CategoryNavA1Preview({ data }: { data: Record<string, any> }) {
    const module = useMemo(() => {
        const merged = JSON.parse(JSON.stringify(data ?? {}));
        mergeDefaultModule(merged, categoryNavA1Defaults);
        return merged;
    }, [data]);

    const commonStyle = formatCommonStyle(module.moduleStyle ?? {}, module.contentStyle ?? {}, module.subTitle ?? {});
    const categoryList: any[] = module.categoryList || [];
    const rowNum = Number(module.rowNum) || 4;

    return (
        <div
            className="module-ad-con h5-catnav-a1"
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
                                    flexBasis: `${100 / rowNum}%`
                                }}
                            >
                                <div className="item-box" style={{ padding: `${module.categoryPadding}px` }}>
                                    <div
                                        className="img-box"
                                        style={{ paddingTop: `${module.picPercent}%`, width: `${module.picPercent}%`, borderRadius: `${module.radius}px` }}
                                    >
                                        <div className="img-box-bg" style={{ backgroundColor: module.imageBackgroundColor }} />
                                        <div
                                            className="category-nav-item-img"
                                            style={{ backgroundImage: `url(${imageFormat(item.picUrl)})`, borderRadius: `${module.radius}px`, backgroundSize: module.picFillType }}
                                        />
                                    </div>
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
                )}
            </div>
        </div>
    );
}
