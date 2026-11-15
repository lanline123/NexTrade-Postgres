/**
 * 分类推荐A1 预览（移植自 view/admin editor/modules/mobilePro/categoryRecA1/Preview.vue）
 *
 * 横排 flex nowrap 四等分卡：底图 background-size=picFillType 铺满 item-inner，
 * 内容区标题（subTitle 三属性）+ 描述（subTitleColor）；空态为灰底占位块。
 * 空态图原版引 empty-img-bg.png 背景图（无字体/图片资源），此处以同尺寸浅灰块承接，文案/数值不变。
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
    const picList: any[] = Array.isArray(module.picList) ? module.picList : [];
    const itemGap = 4;

    return (
        <div
            className="module-ad-con module-category_rec_a1 h5-catrec-a1"
            style={cssText(
                commonStyle.moduleStyle.boxPaddingTop,
                commonStyle.moduleStyle.boxPaddingBottom,
                commonStyle.moduleStyle.boxPadding,
                commonStyle.moduleStyle.backgroundColor,
                commonStyle.moduleStyle.backgroundImage,
                commonStyle.moduleStyle.backgroundSize
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
                {picList.length === 0 ? (
                    <div className="module-ad-empty empty-image_ad">
                        <div className="image-empty-bg" />
                        <div className="desc">点击添加分类</div>
                    </div>
                ) : (
                    <div className="category-rec-a1-list" style={{ paddingLeft: `${itemGap}px` }}>
                        {picList.map((pic, index) => (
                            <div className="category-rec-a1-item" key={index} style={{ flexBasis: "25%", paddingRight: `${itemGap}px` }}>
                                <div className="item-inner" style={{ borderRadius: `${module.picRadius ?? 0}px` }}>
                                    <div
                                        className="item-bg"
                                        style={{ backgroundImage: `url(${imageFormat(pic.picUrl)})`, backgroundSize: module.picFillType || "cover" }}
                                    />
                                    <div className="item-content">
                                        {module.subTitle?.showTitle ? (
                                            <span
                                                className="item-title"
                                                style={cssText(commonStyle.moduleSubTitle.textColor, commonStyle.moduleSubTitle.textFontSize, commonStyle.moduleSubTitle.textBlod)}
                                            >
                                                {pic.picTitle || "标题"}
                                            </span>
                                        ) : null}
                                        <span className="item-desc" style={{ color: module.subTitleColor || "#999999" }}>
                                            {pic.picDesc || "描述"}
                                        </span>
                                    </div>
                                </div>
                            </div>
                        ))}
                    </div>
                )}
            </div>
        </div>
    );
}
