/**
 * 小图滑动 预览（移植自 view/admin editor/modules/mobilePro/slidingImage/Preview.vue）
 *
 * 原版为定时器驱动的无缝轮播（列表复制 3 份、translateX 步进、当前项 scale 1 其余 picReducePercent）；
 * 编辑器内静态承接初始帧：列表复制 3 份横排，偶数位 scale 1、奇数位 picReducePercent（原版 number=0 初态口径）。
 * 注：原版渐变背景表达式两端均取 gradientColorA（源码如此，疑似原版笔误），此处以源码为准。
 */
import { useMemo } from "react";
import { cssText, imageFormat } from "../../../shared/format";
import { mergeDefaultModule, formatCommonStyle, gradientMap } from "../../../contract/sharedDefaults";
import { slidingImageDefaults } from "./index";
import "./Preview.css";

export default function SlidingImagePreview({ data }: { data: Record<string, any> }) {
    const module = useMemo(() => {
        const merged = JSON.parse(JSON.stringify(data ?? {}));
        mergeDefaultModule(merged, slidingImageDefaults);
        return merged;
    }, [data]);

    const commonStyle = formatCommonStyle(module.moduleStyle ?? {}, {}, module.subTitle ?? {});
    const picList: any[] = Array.isArray(module.picList) ? module.picList : [];
    // 原版 copyArray(picList, 3)
    const list = [...picList, ...picList, ...picList];
    const gradientOf = (item: any) =>
        module.backgroundGradientType === "purity"
            ? { backgroundColor: item.gradientColorA }
            : { backgroundImage: `linear-gradient(${gradientMap[module.backgroundGradientType] || "to right bottom"},${item.gradientColorA},${item.gradientColorA})` };

    return (
        <div
            className="module-ad-con h5-sliding-image"
            style={cssText(
                commonStyle.moduleStyle.boxPaddingBottom,
                commonStyle.moduleStyle.boxPaddingTop,
                commonStyle.moduleStyle.boxPadding,
                commonStyle.moduleStyle.backgroundSize,
                commonStyle.moduleStyle.backgroundImage
            )}
        >
            <div className="module-ad-content">
                <div className="slidingImage">
                    <ul className="slidingImage-box">
                        {list.map((item, index) => (
                            <li className="item-box" key={index} style={{ width: `${module.picWidth}%` }}>
                                <div
                                    className="item"
                                    style={{
                                        marginRight: `${module.picMargin}px`,
                                        padding: `${module.picPadding}px`,
                                        borderRadius: `${module.picRadius}px`,
                                        ...gradientOf(item),
                                        transform: `scale(${index % 2 === 0 ? 1 : module.picReducePercent})`
                                    }}
                                >
                                    <div className="fox-title">
                                        <div
                                            className="img"
                                            style={{
                                                backgroundImage: `url(${imageFormat(item.picUrl)})`,
                                                paddingTop: "87.5%",
                                                borderRadius: `${module.picRadius}px`,
                                                backgroundSize: module.picFillType
                                            }}
                                        />
                                        {Number(module.showDesc) === 1 ? (
                                            <div className="desc" style={{ color: module.descColor, ...gradientOf(item) }}>
                                                {item.picDesc}
                                            </div>
                                        ) : null}
                                    </div>
                                    {module.subTitle?.showTitle ? (
                                        <div
                                            className="title"
                                            style={cssText(commonStyle.moduleSubTitle.textFontSize, commonStyle.moduleSubTitle.textBlod, commonStyle.moduleSubTitle.textColor)}
                                        >
                                            {item.picTitle}
                                        </div>
                                    ) : null}
                                </div>
                            </li>
                        ))}
                    </ul>
                </div>
            </div>
        </div>
    );
}
