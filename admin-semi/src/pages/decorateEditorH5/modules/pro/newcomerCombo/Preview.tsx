/**
 * 新人二重礼 预览（移植自 view/admin editor/modules/mobilePro/newcomerCombo/Preview.vue）
 *
 * 头部 + 1 重礼商品横滑卡 + 2 重礼优惠券横排卡。
 * 商品按 products 来源拉取，无来源/无结果时以在售商品池随机真实商品兜底展示
 * （共享 demoProductFill 的 usePreviewDemoProducts，纯显示层不落盘）；池空/
 * 取数失败时走既有空态，不再写死演示商品。演示券 demoCoupons 非商品，不在
 * 演示商品真实化口径内，保留原口径。
 * 优惠券：对齐原版 displayCoupons——条目只存 { couponId, btnText }，券面 info 经
 * useCouponInfoMap 按 couponId 从接口拉取，无 info 的券被过滤，加载中显示
 * 「优惠券信息加载中...」，coupons 为空时用原版演示券兜底。存量旧数据
 * （旧编辑器写入的 couponId=0 + 内嵌 info）在接口无对应券时以其内嵌 info 兜底显示，不至于成空态。
 * 购买钮 1~4 iconfont 字形以语义对应的内联 SVG 承接（同 limitedSpecial 口径）。
 */
import { useEffect, useMemo, useState } from "react";
import { cssText, productImage, priceFormat } from "../../../shared/format";
import { useDecorateProductList } from "../../../shared/useDecorateProductList";
import { useCouponInfoMap } from "../../../shared/useCouponInfoMap";
import { mergeDefaultModule } from "../../../contract/sharedDefaults";
import { usePreviewDemoProducts, demoTargetCount } from "../../../../../components/demoProductFill";
import { resolveFlatBtnColors } from "../../../../../components/buyStyleObj";
import { usePcGlobalBuyStyle } from "../../../../decoratePc/previews/shared/PcBuyButton";
import { newcomerComboDefaults } from "./index";
import "./Preview.css";

const demoCoupons: Array<Record<string, any>> = [
    { couponId: 0, btnText: "立即领取", info: { couponName: "新人专享 满600减40", couponType: 1, couponMoney: 40, minOrderAmount: 600, reduceType: 1 } },
    { couponId: 0, btnText: "立即领取", info: { couponName: "无门槛减5", couponType: 1, couponMoney: 5, minOrderAmount: 0, reduceType: 2 } },
    { couponId: 0, btnText: "立即领取", info: { couponName: "无门槛减20", couponType: 1, couponMoney: 20, minOrderAmount: 0, reduceType: 2 } }
];

const ratioPaddingMap: Record<string, string> = { default: "", "1:1": "100%", "3:4": "133.333%", "3:2": "66.667%" };

const CART_PATH = "M401.3 829.9c-35.8 0-64.9 29.1-64.9 64.9s29.1 64.9 64.9 64.9 64.9-29.1 64.9-64.9-29.1-64.9-64.9-64.9z m400.4 0c-35.8 0-64.9 29.1-64.9 64.9s29.1 64.9 64.9 64.9 64.9-29.1 64.9-64.9-29.1-64.9-64.9-64.9z m211.9-622.1c-11-12.7-27.2-19.9-44.1-19.9H238.9l-17.4-96.8C217.4 55.4 185.5 29 149.5 29H63.4C37.8 29 17 49.8 17 75.4s20.8 46.4 46.4 46.4h86.1c3.2 0 5.9 2.3 6.5 5.4l76.7 425.7c8.5 47.6 49.8 82.1 98.2 82.1h438.3c46.4 0 86.9-33 96.2-78.5l54.7-270.4c3.4-18.1-1.3-36.6-13.5-50.3z";
const CART2_PATH = "M896 149.3H725.3l-27.7-55.5c-8.1-16.2-24.6-26.5-42.7-26.5H369.1c-18.1 0-34.6 10.2-42.7 26.5l-27.7 55.5H128c-35.3 0-64 28.7-64 64v618.7c0 35.3 28.7 64 64 64h768c35.3 0 64-28.7 64-64V213.3c0-35.3-28.7-64-64-64z m-64 576H192V320h640v405.3z";
const PLUS_PATH = "M448 128h128v320h320v128H576v320H448V576H128V448h320V128z";
const PLUS2_PATH = "M512 64C264.6 64 64 264.6 64 512s200.6 448 448 448 448-200.6 448-448S759.4 64 512 64z m213.3 480H544v181.3c0 17.6-14.4 32-32 32s-32-14.4-32-32V544H298.7c-17.6 0-32-14.4-32-32s14.4-32 32-32H480V298.7c0-17.6 14.4-32 32-32s32 14.4 32 32V480h181.3c17.6 0 32 14.4 32 32s-14.4 32-32 32z";

function BuyBtnIcon({ style }: { style: number }) {
    const path = style === 1 ? CART_PATH : style === 2 ? CART2_PATH : style === 3 ? PLUS_PATH : PLUS2_PATH;
    return (
        <span className="nc-buy-icon">
            <svg viewBox="0 0 1024 1024" width="1em" height="1em" fill="currentColor" style={{ display: "block" }}>
                <path d={path} />
            </svg>
        </span>
    );
}

export default function NewcomerComboPreview({ data }: { data: Record<string, any> }) {
    const module = useMemo(() => {
        const merged = JSON.parse(JSON.stringify(data ?? {}));
        mergeDefaultModule(merged, newcomerComboDefaults);
        return merged;
    }, [data]);

    const fetched = useDecorateProductList(module.products);
    const demoFallback = usePreviewDemoProducts(fetched.length === 0, demoTargetCount("newcomerCombo"));
    const productList = fetched.length > 0 ? fetched : demoFallback;

    const coupons: Array<Record<string, any>> = Array.isArray(module.coupons) ? module.coupons : [];
    const { couponInfoMap, fetchCouponInfo } = useCouponInfoMap();
    const [couponInfoLoading, setCouponInfoLoading] = useState(false);
    const couponIdsKey = coupons
        .map((c) => Number(c.couponId) || 0)
        .filter((id) => id > 0)
        .sort((a, b) => a - b)
        .join(",");
    useEffect(() => {
        if (!couponIdsKey) return;
        let alive = true;
        setCouponInfoLoading(true);
        fetchCouponInfo(couponIdsKey.split(",").map(Number)).finally(() => {
            if (alive) setCouponInfoLoading(false);
        });
        return () => {
            alive = false;
        };
    }, [couponIdsKey, fetchCouponInfo]);

    // 原版：coupons 非空时按 couponId 取接口 info 并过滤无 info 项；接口 info 优先，
    // 存量内嵌 info（旧编辑器数据）仅在接口无此券时兜底，避免存量页面预览成空
    const displayCoupons =
        coupons.length === 0
            ? demoCoupons
            : coupons
                  .map((c) => ({ ...c, info: couponInfoMap.get(Number(c.couponId)) || c.info || null }))
                  .filter((c) => c.info);

    const f = module.frame ?? {};
    const outerStyle = cssText(
        `padding-left:${f.boxPadding ?? 0}px;`,
        `padding-right:${f.boxPadding ?? 0}px;`,
        `padding-top:${f.boxPaddingTop ?? 0}px;`,
        `padding-bottom:${f.boxPaddingBottom ?? 0}px;`,
        f.backgroundColor ? `background-color:${f.backgroundColor};` : ""
    );

    const bgImg = module.containerBgImage?.picUrl;
    const layers: string[] = [];
    const sizes: string[] = [];
    if (bgImg) {
        layers.push(`url(${bgImg})`);
        sizes.push("cover");
    }
    layers.push(`linear-gradient(to bottom, ${module.boxGradientStart || "#FF3630"}, ${module.boxGradientEnd || "#FF7A45"})`);
    sizes.push("auto");
    const containerBgStyle = cssText(
        `background-image:${layers.join(",")};`,
        `background-size:${sizes.join(",")};`,
        "background-position:center;",
        "background-repeat:no-repeat;",
        `border-radius:${f.boxRadius ?? 8}px;`
    );

    const badgeStyle = cssText(
        `background-color:${module.productBriefGradientStart || "#FF3630"};`,
        `color:${module.productBriefTextColor || "#FFFFFF"};`
    );
    const briefStyle = cssText(
        `background-image:linear-gradient(to bottom, ${module.productBriefGradientStart || "#FF3630"}, ${module.productBriefGradientEnd || "#FF7A45"});`,
        `color:${module.productBriefTextColor || "#FFFFFF"};`
    );

    const ratio = module.productPicRatio || "1:1";
    const ratioPadding = ratioPaddingMap[ratio] ?? "";
    const photoStyle = cssText(ratioPadding ? `padding-top:${ratioPadding};` : "padding-top:0;");
    const imgWrapStyle = cssText(`border-radius:${module.productImgRadius ?? 8}px;`, ratioPadding ? "position:absolute;inset:0;" : "position:static;");
    const imgStyle = cssText(
        `object-fit:${module.productPicFillType || "cover"};`,
        ratioPadding ? "position:absolute;inset:0;width:100%;height:100%;display:block;" : "width:100%;height:auto;display:block;"
    );
    const itemStyle = cssText(
        module.contentBgColor ? `background-color:${module.contentBgColor};` : "",
        `padding:${module.productItemPadding ?? 6}px;box-sizing:border-box;`,
        `flex:0 0 ${module.productPercent ?? 28}%;width:${module.productPercent ?? 28}%;`
    );
    const trackStyle = cssText(`gap:${module.productItemGap ?? 8}px;`);

    const pillBg = module.couponBgGradientStart || "#FFF1F1";
    const moreLinkPillStyle = cssText(
        `background-color:${pillBg === "#FFF1F1" ? "#FFEAD8" : pillBg};`,
        `color:${module.couponBtnTextColor || "#FF3630"};`
    );
    // 按钮三色：扁平色族跟随解析（三期，与 H5 渲染同口径）
    const globalBuyStyle = usePcGlobalBuyStyle();
    const flatBtn = resolveFlatBtnColors(module, globalBuyStyle, { gradientStart: "#4CD964", gradientEnd: "#2EBA4F", textColor: "#FFFFFF" });
    const buyBtnStyleObj = cssText(
        `background-image:linear-gradient(to right, ${flatBtn.gradientStart}, ${flatBtn.gradientEnd});`,
        `color:${flatBtn.textColor};`
    );
    const couponItemStyle = ((): React.CSSProperties => {
        const n = module.couponPerRow || 3;
        const gap = 8;
        const basis = `calc((100% - ${(n - 1) * gap}px) / ${n})`;
        return cssText(
            `background-image:linear-gradient(to bottom, ${module.couponBgGradientStart || "#FFF1F1"}, ${module.couponBgGradientEnd || "#FFFFFF"});`,
            `flex:0 0 ${basis};max-width:${basis};`,
            `--dec-bg:${module.decorationBgColor || "#FFFFFF"};`
        );
    })();

    const buyBtn = Number(module.buyBtnStyle ?? 0);

    return (
        <div className="module-ad-con module-newcomer_combo" style={outerStyle}>
            <div className="nc-combo-wrap" style={containerBgStyle}>
                {Number(module.showHeader) === 1 ? (
                    <div className="nc-combo-header">
                        <span className="nc-header-title">{module.headerTitle}</span>
                        {module.headerSubTitle ? <span className="nc-header-sub">（{module.headerSubTitle}）</span> : null}
                    </div>
                ) : null}

                {Number(module.showSection1) === 1 ? (
                    <div className="nc-combo-card" style={{ backgroundColor: module.decorationBgColor }}>
                        <div className="nc-card-head">
                            <span className="nc-card-badge" style={badgeStyle}>{module.section1Badge}</span>
                            <span className="nc-card-title" style={{ color: module.productTitleColor }}>{module.section1Title}</span>
                            <a className="nc-more-link" style={{ color: module.productBriefColor || "#999999" }}>查看更多</a>
                        </div>
                        {!productList || productList.length === 0 ? (
                            <div className="nc-empty"><span>请先选择商品</span></div>
                        ) : (
                            <div className="nc-product-scroll">
                                <div className="nc-product-track" style={trackStyle}>
                                    {productList.map((item: any, idx: number) => (
                                        <div className="nc-product-item" key={`${item.productId || 0}_${idx}`} style={itemStyle}>
                                            <div className="nc-product-photo" style={photoStyle}>
                                                <div className="nc-product-img-wrap" style={imgWrapStyle}>
                                                    <img src={productImage(item)} alt="" style={imgStyle} />
                                                </div>
                                                {module.productBriefList?.[idx] ? (
                                                    <div className="nc-product-brief" style={briefStyle}>{module.productBriefList[idx]}</div>
                                                ) : null}
                                            </div>
                                            {Number(module.showProductName) === 1 ? (
                                                <div className="nc-product-name" style={{ color: module.productTitleColor }}>{item.productName}</div>
                                            ) : null}
                                            <div className="nc-product-bottom">
                                                <div className="nc-product-prices">
                                                    <span className="nc-price-current" style={{ color: module.productPriceColor }}>
                                                        <i>¥</i><b>{priceFormat(Number(item.productPrice))}</b>
                                                    </span>
                                                    {Number(module.showMarketPrice) === 1 && Number(item.marketPrice) > 0 && Number(item.marketPrice) > Number(item.productPrice) ? (
                                                        <span className="nc-price-market" style={{ color: module.productMarketPriceColor }}>
                                                            ¥{priceFormat(Number(item.marketPrice))}
                                                        </span>
                                                    ) : null}
                                                </div>
                                                {buyBtn > 0 ? (
                                                    <a className={`nc-buy-btn nc-buy-btn-style__${buyBtn}`} style={buyBtnStyleObj}>
                                                        {buyBtn === 5 || buyBtn === 6 ? <span>购买</span> : buyBtn === 7 || buyBtn === 8 ? <span>马上抢</span> : <BuyBtnIcon style={buyBtn} />}
                                                    </a>
                                                ) : null}
                                            </div>
                                        </div>
                                    ))}
                                </div>
                            </div>
                        )}
                    </div>
                ) : null}

                {Number(module.showSection2) === 1 ? (
                    <div className="nc-combo-card" style={{ backgroundColor: module.decorationBgColor }}>
                        <div className="nc-card-head">
                            <span className="nc-card-badge" style={badgeStyle}>{module.section2Badge}</span>
                            <span className="nc-card-title" style={{ color: module.productTitleColor }}>{module.section2Title}</span>
                            <a className="nc-more-link nc-more-link-pill" style={moreLinkPillStyle}>查看更多</a>
                        </div>
                        {displayCoupons.length ? (
                            <div className="nc-coupon-row">
                                {displayCoupons.map((c: any, i: number) => (
                                    <div className="nc-coupon-item" key={c.couponId || i} style={couponItemStyle}>
                                        <div className="nc-coupon-amount" style={{ color: module.couponAmountColor }}>
                                            {Number(c.info.couponType) === 2 ? (
                                                <>
                                                    <b>{c.info.couponDiscount}</b><i className="nc-amount-prefix">折</i>
                                                </>
                                            ) : (
                                                <>
                                                    <i className="nc-amount-prefix">¥</i><b>{c.info.couponMoney}</b>
                                                </>
                                            )}
                                        </div>
                                        <div className="nc-coupon-condition" style={{ color: module.couponConditionColor }}>
                                            {Number(c.info.reduceType) === 2 || !Number(c.info.minOrderAmount) ? <span>无门槛</span> : <span>满 {c.info.minOrderAmount} 可用</span>}
                                        </div>
                                    </div>
                                ))}
                            </div>
                        ) : couponInfoLoading ? (
                            <div className="nc-empty">优惠券信息加载中...</div>
                        ) : (
                            <div className="nc-empty">请在右侧选择优惠券</div>
                        )}
                    </div>
                ) : null}
            </div>
        </div>
    );
}
