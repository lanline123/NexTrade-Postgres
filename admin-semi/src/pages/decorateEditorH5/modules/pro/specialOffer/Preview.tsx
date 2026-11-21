/**
 * 天天特价 预览（对齐 view/admin mobilePro/specialOffer/Preview.vue）
 * - 商品列表口径与原版一致：手动 = productList 中演示项（productId 0）+ 按 ids 拉取的真实商品；
 *   分组 = productGroups[0] 按 productGroupId 拉取（size 口径同原版）；拉取走现有 admin API；
 * - 顶部右侧：countdown（CommonCountdown→H5Countdown，文案 now>dailyTime[0] 时为 countdownTitle
 *   否则「距开始」，原版对 specifyTime 模式同样按 dailyTime[0] 比较，照此保留）或 button
 *   （渐变取 moduleColor.buttonGradient* 优先，空则 ModuleTheme[moduleColorStyle] 预设）；
 * - 自定义标题/卖点区（goods-info）仅当 prodcutTitles 非空时渲染（原版 v-if 口径）；
 *   字段名 prodcutTitles/showProdcutTitles 及逐项 title{n}/sellingPoints{n} 键名逐字保留；
 * - 价格框使用原版固定背景图，价格为原始值直出（原版未走 priceFormat）；
 * - moduleTitle/moduleColor 嵌套新增字段（topRightType 等）按原版 Preview 的显式兜底逻辑补齐。
 */
import "./Preview.css";
import { useEffect, useMemo, useState } from "react";
import { mergeDefaultModule, cloneDeep, formatCommonStyle } from "../../../contract/sharedDefaults";
import { cssText, productImage } from "../../../shared/format";
import { apiGet } from "../../../../../api";
import H5Countdown from "../../../shared/H5Countdown";
import { specialOfferDefaults } from "./index";

interface Props {
    data: Record<string, any>;
}

/** 原版 ModuleTheme（editIndex.ts）预设：仅取按钮渐变兜底所需字段 */
export const SPECIAL_OFFER_THEMES: Record<string, { textColor: string; gradientColorA: string; gradientColorB: string }> = {
    orange: { textColor: "#FF3E02", gradientColorA: "#ff3e02", gradientColorB: "#ff7b19" },
    red: { textColor: "#FF1513", gradientColorA: "#ff1513", gradientColorB: "#ff856c" },
    blue: { textColor: "#4499E6", gradientColorA: "#4499e6", gradientColorB: "#7faff5" },
    green: { textColor: "#59c12d", gradientColorA: "#59c12d", gradientColorB: "#64d135" },
    purple: { textColor: "#7649d7", gradientColorA: "#7649d7", gradientColorB: "#8456e3" },
    grey: { textColor: "#373737", gradientColorA: "#373737", gradientColorB: "#6d6d6d" },
    chocolate: { textColor: "#4e362e", gradientColorA: "#4e362e", gradientColorB: "#603929" },
    brown: { textColor: "#a67e5a", gradientColorA: "#a67e5a", gradientColorB: "#c19270" }
};

const PIC_SCALE_MAP: Record<string, string> = {
    "3:2": "calc(100% / 3 * 2)",
    "1:1": "calc(100% / 1 * 1)",
    "3:4": "calc(100% / 3 * 4)",
    "16:9": "calc(100% / 16 * 9)"
};

export default function SpecialOfferPreview({ data }: Props) {
    const m = useMemo(() => {
        const merged = JSON.parse(JSON.stringify(data ?? {}));
        mergeDefaultModule(merged, cloneDeep(specialOfferDefaults));
        // 原版对嵌套新增字段的显式兜底（浅合并不会回填已存在嵌套对象的新增字段）
        if (merged.moduleTitle) {
            if (merged.moduleTitle.topRightType === undefined) merged.moduleTitle.topRightType = "countdown";
            if (merged.moduleTitle.buttonText === undefined) merged.moduleTitle.buttonText = "立即抢购";
            if (merged.moduleTitle.buttonLink === undefined) merged.moduleTitle.buttonLink = {};
        }
        if (merged.moduleColor) {
            if (merged.moduleColor.buttonTextColor === undefined) merged.moduleColor.buttonTextColor = "#ffffff";
            if (merged.moduleColor.buttonGradientColorA === undefined) merged.moduleColor.buttonGradientColorA = "";
            if (merged.moduleColor.buttonGradientColorB === undefined) merged.moduleColor.buttonGradientColorB = "";
        }
        return merged;
    }, [data]);

    const sourceKey = useMemo(() => {
        if (m.productSource === "products") {
            return `products:${(m.productList ?? []).map((p: any) => p.productId).join(",")}`;
        }
        if (m.productSource === "all") {
            return `all:${m.productNum ?? 100}`;
        }
        const g = m.productGroups?.[0];
        return g ? `group:${g.productGroupId}:${g.productNumType}:${g.productNum}` : "none";
    }, [m.productSource, m.productList, m.productGroups, m.productNum]);

    const [fetched, setFetched] = useState<Array<Record<string, any>>>([]);
    useEffect(() => {
        let cancelled = false;
        (async () => {
            try {
                if (m.productSource === "products") {
                    const ids = (m.productList ?? []).map((p: any) => p.productId).filter((id: any) => Number(id) > 0);
                    if (!ids.length) {
                        if (!cancelled) setFetched([]);
                        return;
                    }
                    const res: any = await apiGet("/product/product/list", { ids: ids.join(",") });
                    if (!cancelled) setFetched(Array.isArray(res?.records) ? res.records : []);
                } else if (m.productSource === "all") {
                    // 全部商品（我方新增）：仅 size，无任何过滤（口径同店面渲染端）
                    const res: any = await apiGet("/product/product/list", {
                        size: Number(m.productNum) > 0 ? Number(m.productNum) : 100
                    });
                    if (!cancelled) setFetched(Array.isArray(res?.records) ? res.records : []);
                } else if (m.productSource === "productGroup" && m.productGroups?.[0]) {
                    const { productGroupId, productNum, productNumType } = m.productGroups[0];
                    const res: any = await apiGet("/product/product/list", {
                        productGroupId,
                        size: productNumType === "number" ? productNum : 100
                    });
                    if (!cancelled) setFetched(Array.isArray(res?.records) ? res.records : []);
                } else if (!cancelled) {
                    setFetched([]);
                }
            } catch {
                if (!cancelled) setFetched([]);
            }
        })();
        return () => {
            cancelled = true;
        };
        // eslint-disable-next-line react-hooks/exhaustive-deps
    }, [sourceKey]);

    const list = useMemo(() => {
        if (m.productSource === "products") {
            const demo = (m.productList ?? []).filter((p: any) => !Number(p.productId));
            return [...demo, ...fetched];
        }
        return fetched;
    }, [m.productSource, m.productList, fetched]);

    const common = formatCommonStyle(m.moduleStyle ?? {}, m.contentStyle ?? {}, m.subTitle ?? {});
    const outerStyle = cssText(common.moduleStyle.boxPaddingBottom + common.moduleStyle.boxPaddingTop + common.moduleStyle.boxPadding + common.moduleStyle.backgroundSize + common.moduleStyle.backgroundImage);
    const contentStyle = cssText(
        common.moduleContentStyle.backgroundImage +
            common.moduleContentStyle.backgroundSize +
            common.moduleContentStyle.boxRadius +
            common.moduleContentStyle.boxShadow +
            common.moduleContentStyle.innerPadding +
            `border-radius: ${m.contentStyle?.boxRadius ?? 0}px;`
    );

    const mt = m.moduleTitle ?? {};
    const mc = m.moduleColor ?? {};
    const topRightType = mt.topRightType ?? "countdown";
    const theme = SPECIAL_OFFER_THEMES[m.moduleColorStyle || "orange"];
    const btnA = mc.buttonGradientColorA || theme?.gradientColorA || "#ff3e02";
    const btnB = mc.buttonGradientColorB || theme?.gradientColorB || "#ff7b19";
    const titles: Array<Record<string, any>> = Array.isArray(m.prodcutTitles) ? m.prodcutTitles : [];

    return (
        <div className="spo-root" style={outerStyle}>
            <div className="spo-content" style={contentStyle}>
                <div className="spo-main">
                    <div className="spo-mk-top">
                        <div>
                            <span className="spo-mk-title" style={cssText(`color: ${mc.titleColor};`)}>
                                {mt.title}
                            </span>
                            <span className="spo-mk-small-title" style={cssText(`color: ${mc.subTitleColor};`)}>
                                {mt.subTitle}
                            </span>
                        </div>
                        {topRightType === "countdown" ? (
                            <div className="spo-mk-countdown" style={cssText(`--spo-color: ${mc.countdownColor};`) as React.CSSProperties}>
                                <span className="spo-cd-text" style={cssText(`color: ${mc.countdownTitleColor};`)}>
                                    {Date.now() > Number(mt.dailyTime?.[0]) ? mt.countdownTitle : "距开始"}
                                </span>
                                <div className="spo-cd-box">
                                    <H5Countdown
                                        countdownType={mt.countdownType}
                                        dailyTime={mt.dailyTime}
                                        specifyTime={mt.specifyTime}
                                        format="HH:mm:ss"
                                        valueStyle={{ color: mc.countdownColor, fontSize: "12px", display: "block", height: "20px", lineHeight: "20px" }}
                                    />
                                </div>
                            </div>
                        ) : (
                            <div
                                className="spo-mk-action-btn"
                                style={cssText(`background: linear-gradient(90deg, ${btnA} 0%, ${btnB} 100%); color: ${mc.buttonTextColor || "#ffffff"};`)}
                            >
                                <span className="spo-btn-text">{mt.buttonText || "立即抢购"}</span>
                                <span aria-hidden="true">›</span>
                            </div>
                        )}
                    </div>
                    <div className="spo-lists" style={cssText(`border-radius: ${m.contentStyle?.boxRadius ?? 0}px;`)}>
                        <div className="spo-item-box">
                            {list.map((item, index) => (
                                <div key={index} style={{ width: `${m.productWidth}%`, flexWrap: "nowrap", flexShrink: 0 } as React.CSSProperties}>
                                    <div className="spo-item">
                                        <div
                                            className="spo-img"
                                            style={cssText(
                                                `width: 100%; padding-top: ${PIC_SCALE_MAP[m.picScale] ?? "100%"}; background-size: ${m.picFillType}; border-radius: ${m.productRadius}px; background-image: url(${productImage(item)});`
                                            )}
                                        />
                                        {titles.length > 0 ? (
                                            <div className="spo-goods-info">
                                                {Number(m.showProductName) === 1 && Number(m.showProdcutTitles) === 1 ? (
                                                    <div
                                                        className="spo-tit"
                                                        style={cssText(
                                                            `padding-bottom: 0px; font-size: ${m.productNameSize}px; font-weight: ${m.productNameWeight}; color:${mc.productNameColor};`
                                                        )}
                                                    >
                                                        {titles[index]?.[`title${index + 1}`]}
                                                    </div>
                                                ) : null}
                                                {Number(m.showProductPrice) === 1 ? (
                                                    <div className="spo-price-box" style={cssText("color: rgb(255, 255, 255); font-weight: 700; font-family: inherit;")}>
                                                        <div className="spo-price">￥{item.productPrice}</div>
                                                    </div>
                                                ) : null}
                                                {Number(m.showSellingPoints) === 1 && Number(m.showProdcutTitles) === 1 ? (
                                                    <div className="spo-desc" style={cssText(`color: ${mc.sellingPointsColor};`)}>
                                                        {titles[index]?.[`sellingPoints${index + 1}`]}
                                                    </div>
                                                ) : null}
                                            </div>
                                        ) : null}
                                    </div>
                                </div>
                            ))}
                        </div>
                    </div>
                </div>
            </div>
        </div>
    );
}
