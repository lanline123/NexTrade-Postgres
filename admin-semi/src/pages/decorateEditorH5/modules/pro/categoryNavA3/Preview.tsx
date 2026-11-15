/**
 * 类目导航A3 预览（移植自 view/admin editor/modules/mobilePro/categoryNavA3/Preview.vue）
 *
 * 与 A1 差异：文字行 margin-top 8px（A1 为 3px）；每项另有描述行（logoPic 图标 16×16 + picDesc）；
 * item-box 仅左内距 categoryPadding，slide-box 整体 translateX(-categoryPadding)。
 * 原版以容器实测宽度算每项 px 宽；编辑器内以 flex-basis 百分比等价承接（100/rowNum %）。
 */
import { useMemo } from "react";
import { cssText, imageFormat } from "../../../shared/format";
import { mergeDefaultModule, formatCommonStyle } from "../../../contract/sharedDefaults";
import { categoryNavA3Defaults } from "./index";
import "./Preview.css";

export default function CategoryNavA3Preview({ data }: { data: Record<string, any> }) {
    const module = useMemo(() => {
        const merged = JSON.parse(JSON.stringify(data ?? {}));
        mergeDefaultModule(merged, categoryNavA3Defaults);
        return merged;
    }, [data]);

    const commonStyle = formatCommonStyle(module.moduleStyle ?? {}, module.contentStyle ?? {}, module.subTitle ?? {});
    const categoryList: any[] = module.categoryList || [];
    const rowNum = Number(module.rowNum) || 4;

    return (
        <div
            className="module-ad-con h5-catnav-a3"
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
                <div className="category-nav">
                    <div
                        className={`decorate-slide-box ${module.listStyleType}`}
                        style={{ transform: `translate(${-Number(module.categoryPadding) || 0}px, 0px)` }}
                    >
                        {categoryList.map((item, index) => (
                            <div className="category-nav-item" key={index} style={{ flexShrink: 0, flexBasis: `${100 / rowNum}%` }}>
                                <div className="item-box" style={{ paddingLeft: `${module.categoryPadding}px` }}>
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
                                    <div className="category-nav-item-desc">
                                        {module.logoPic?.picUrl ? <img className="desc-icon" src={imageFormat(module.logoPic.picUrl)} alt="" /> : null}
                                        <div className="desc-text">{item.picDesc}</div>
                                    </div>
                                </div>
                            </div>
                        ))}
                    </div>
                </div>
            </div>
        </div>
    );
}
