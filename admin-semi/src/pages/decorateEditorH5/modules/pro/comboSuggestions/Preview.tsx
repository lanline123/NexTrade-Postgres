/**
 * 组合推荐 预览（移植自 view/admin editor/modules/mobilePro/comboSuggestions/Preview.vue）
 *
 * comboGroups 逐组双列排布（每组宽 calc(50% - 4px)，第 3 组起 margin-top 10px）：
 * 组头 = 头图或大标题（15px/700）+ 徽标（groupColor 底、11px 白字）+ 描述行；
 * image 组为轮播图（高按原版公式 (容器宽/2 - 16) * 0.63，以 aspect-ratio 等价承接）+ 右下计数；
 * product 组为横排商品（原版 Swiper slidesPerView 2、spaceBetween 6），图 padding-top 100%、价格签居中。
 * 商品数据直接用分组内 pruductList 快照渲染（原版对 productId=0 的演示项即走 getDemoList 同口径）。
 */
import { useMemo } from "react";
import { cssText, imageFormat, productImage } from "../../../shared/format";
import { mergeDefaultModule, formatCommonStyle, gradientMap } from "../../../contract/sharedDefaults";
import { comboSuggestionsDefaults } from "./index";
import "./Preview.css";

export default function ComboSuggestionsPreview({ data }: { data: Record<string, any> }) {
    const module = useMemo(() => {
        const merged = JSON.parse(JSON.stringify(data ?? {}));
        mergeDefaultModule(merged, comboSuggestionsDefaults);
        return merged;
    }, [data]);

    const commonStyle = formatCommonStyle(module.moduleStyle ?? {});
    const groups: any[] = Array.isArray(module.comboGroups) ? module.comboGroups : [];
    const listStyle = `border-radius: ${module.groupRadius}px;background-image: linear-gradient(${gradientMap[module.backgroundGradientType] || "to bottom"},${module.gradientColorA} -50%, ${module.gradientColorB});${
        Number(module.groupShadow) === 1 ? "box-shadow: rgba(0, 0, 0, 0.06) 0px 2px 4px;" : ""
    }`;

    return (
        <div
            className="module-ad-con h5-combo-sug"
            style={cssText(
                commonStyle.moduleStyle.boxPaddingBottom,
                commonStyle.moduleStyle.boxPaddingTop,
                commonStyle.moduleStyle.boxPadding,
                commonStyle.moduleStyle.backgroundSize,
                commonStyle.moduleStyle.backgroundImage
            )}
        >
            <div className="main">
                {groups.length === 0 ? (
                    <div className="module-ad-empty empty-image_ad">
                        <div className="image-empty-bg" />
                        <div className="desc">点击添加组合推荐</div>
                    </div>
                ) : null}
                {groups.map((group, index) => (
                    <div className="list" key={index} style={{ ...cssText(listStyle), ...(index > 1 ? { marginTop: 10 } : {}) }}>
                        <div style={{ padding: 6 }}>
                            <div className="top">
                                <div className="top-tit">
                                    {group.titleImage?.picUrl ? (
                                        <img className="img" src={imageFormat(group.titleImage.picUrl)} alt="" />
                                    ) : (
                                        <div className="big-title" style={{ color: module.groupTitleColor }}>
                                            {group.groupTitle}
                                        </div>
                                    )}
                                    <div className="title" style={{ background: group.groupColor }}>
                                        {group.groupSubTitle}
                                    </div>
                                </div>
                                <div className="desc" style={{ color: group.groupColor }}>
                                    {group.groupDesc}
                                </div>
                            </div>
                            <div style={{ borderRadius: `${module.groupRadius}px`, overflow: "hidden", position: "relative" }}>
                                {group.groupType === "image" ? (
                                    <>
                                        {(group.picList || []).slice(0, 1).map((item: any, i: number) => (
                                            <div className="pic-box" key={i}>
                                                <div
                                                    className="pic-img"
                                                    style={{ backgroundImage: `url(${imageFormat(item.picUrl)})`, aspectRatio: "1 / 0.63" }}
                                                />
                                                <div className="tit" style={{ color: module.bannerTittleColor }}>
                                                    {item.picTitle}
                                                </div>
                                            </div>
                                        ))}
                                        {(group.picList || []).length > 0 ? (
                                            <div style={{ position: "absolute", right: 10, bottom: 5, lineHeight: "18px", color: module.bannerDotColor, zIndex: 9 }}>
                                                {`1/${(group.picList || []).length}`}
                                            </div>
                                        ) : null}
                                    </>
                                ) : null}
                                {group.groupType === "product" ? (
                                    <div className="product-row">
                                        {(group.pruductList || []).map((item: any, i: number) => (
                                            <div className="item" key={i}>
                                                <div
                                                    className="goods-img"
                                                    style={{
                                                        width: "100%",
                                                        paddingTop: "100%",
                                                        backgroundSize: "cover",
                                                        borderRadius: `${module.imageRadius}px`,
                                                        backgroundImage: `url(${productImage(item)})`
                                                    }}
                                                />
                                                <div className="goods-price">
                                                    <span
                                                        className="price-text"
                                                        style={{
                                                            color: module.regularPriceColor,
                                                            background: module.regularPriceBgColor || "transparent",
                                                            borderRadius: `${module.regularPriceRadius}px`,
                                                            fontSize: `${module.regularPriceSize}px`
                                                        }}
                                                    >
                                                        ￥{item.productPrice}
                                                    </span>
                                                </div>
                                            </div>
                                        ))}
                                    </div>
                                ) : null}
                            </div>
                        </div>
                    </div>
                ))}
            </div>
        </div>
    );
}
