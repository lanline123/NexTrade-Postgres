/**
 * 商品滑动 预览（对齐 view/admin mobilePro/productSlide/Preview.vue）
 * - 按 module.products 来源拉真实商品（useDecorateProductList），无来源/拉取为空时以在售商品池
 *   随机真实商品兜底展示（共享 demoProductFill 的 usePreviewDemoProducts，纯显示层不落盘），
 *   池空/取数失败时列表为空（诚实空态），不再写死演示商品；
 * - 逐卡渐变底（cardGradientStartList/EndList，缺省走原版兜底调色板）、逐卡大标题、
 *   图片比例盒（padding-top 占位）、图片平台阴影、卖点徽标、价格/原价/销量、数字购买钮；
 * - 购买钮 1~4 的 iconfont 以内联 SVG 等价承接（gouwuche/gouwuche1/jia/jia1），5/6「购买」7/8「马上抢」；
 * - 外层 frame 边距/背景按原版 outerStyle 手工口径（boxPadding 家族 + moduleBgImage），
 *   不走 formatFrame（原版即手工拼接）。
 */
import "./Preview.css";
import { useMemo } from "react";
import { mergeDefaultModule, cloneDeep, defaultProductSlide } from "../../../contract/sharedDefaults";
import { cssText, productImage, priceFormat } from "../../../shared/format";
import { useDecorateProductList } from "../../../shared/useDecorateProductList";
import { usePreviewDemoProducts, demoTargetCount } from "../../../../../components/demoProductFill";
import { resolveFlatBtnColors } from "../../../../../components/buyStyleObj";
import { usePcGlobalBuyStyle } from "../../../../decoratePc/previews/shared/PcBuyButton";

interface Props {
    data: Record<string, any>;
}

const GRADIENT_FALLBACK: Array<[string, string]> = [
    ["#B6A8F5", "#8E7BF0"],
    ["#FF8FB8", "#FF5E96"],
    ["#FFD98A", "#FBC25A"],
    ["#C7BEF8", "#9C8CF0"]
];

const RATIO_PADDING: Record<string, string> = {
    default: "",
    "1:1": "100%",
    "3:4": "133.333%",
    "3:2": "66.667%"
};

const CART_PATH = "M832 288H200.7l-8.1-40.5C190.9 239.2 183.7 232 175.2 232H96a16 16 0 0 0 0 32h66l74 371.9C238.7 660.9 256 672 272 672h504.4c14.9 0 27.9-9.9 30.7-23.8l54.3-270.9C863.2 392.2 853.6 288 832 288zM761.5 640H281.2l-9.5-47.5h505.1l-15.3 47.5zM296 768a56 56 0 1 0 0 112 56 56 0 0 0 0-112zm400 0a56 56 0 1 0 0 112 56 56 0 0 0 0-112z";
const CART2_PATH = "M912 320H224.4L208 152.7A64 64 0 0 0 144.3 96H64a32 32 0 1 0 0 64h48.2l75.4 756.8A64 64 0 0 0 251.3 976h620.1a32 32 0 1 0 0-64H267.2l-9.6-96h592.3a64 64 0 0 0 62.8-54.5L953.6 395c2.9-39.7-17.9-75-41.6-75zM336 832a64 64 0 1 1 0-128 64 64 0 0 1 0 128zm448 0a64 64 0 1 1 0-128 64 64 0 0 1 0 128z";
const PLUS_PATH = "M480 480V128a32 32 0 0 1 64 0v352h352a32 32 0 0 1 0 64H544v352a32 32 0 1 1-64 0V544H128a32 32 0 0 1 0-64h352z";
const PLUS2_PATH = "M512 64a448 448 0 1 0 0 896 448 448 0 0 0 0-896zm224 480H544v192a32 32 0 1 1-64 0V544H288a32 32 0 0 1 0-64h192V288a32 32 0 1 1 64 0v192h192a32 32 0 0 1 0 64z";

const BUY_ICON_PATHS: Record<number, string> = { 1: CART_PATH, 2: CART2_PATH, 3: PLUS_PATH, 4: PLUS2_PATH };

function cardStyleBg(cardStyle: string): string {
    switch (cardStyle) {
        case "noBorderWhite":
            return "background-color:#ffffff;";
        case "shadow":
            return "background-color:#ffffff;box-shadow:0 4px 12px rgba(0,0,0,0.12);";
        case "border":
            return "background-color:#ffffff;border:1px solid rgba(0,0,0,0.08);";
        default:
            return "background-color:transparent;";
    }
}

export default function ProductSlidePreview({ data }: Props) {
    const m = useMemo(() => { const merged = JSON.parse(JSON.stringify(data ?? {})); mergeDefaultModule(merged, cloneDeep(defaultProductSlide)); return merged; }, [data]);
    const fetched = useDecorateProductList(m.products);
    const demoFallback = usePreviewDemoProducts(fetched.length === 0, demoTargetCount("productSlide"));
    const list = fetched.length > 0 ? fetched : demoFallback;

    const f = m.frame ?? {};
    const bgImg = m.moduleBgImage?.picUrl;
    const outerStyle = cssText(
        [
            `padding-left:${f.boxPadding ?? 0}px;`,
            `padding-right:${f.boxPadding ?? 0}px;`,
            `padding-top:${f.boxPaddingTop ?? 0}px;`,
            `padding-bottom:${f.boxPaddingBottom ?? 0}px;`,
            f.backgroundColor ? `background-color:${f.backgroundColor};` : "",
            bgImg ? `background-image:url(${bgImg});background-size:cover;background-position:center;background-repeat:no-repeat;` : ""
        ].join("")
    );
    const wrapStyle = cssText(`background-color:${m.innerBgColor || "transparent"};border-radius:${f.boxRadius ?? 16}px;`);

    const align = m.textAlign || "center";
    const itemsAlign = align === "left" ? "flex-start" : align === "right" ? "flex-end" : "center";
    const ratio = m.productPicRatio || "1:1";
    const ratioPad = RATIO_PADDING[ratio] ?? "";
    const imgWrapStyle = cssText(`${cardStyleBg(m.cardStyle)}border-radius:${m.productImgRadius ?? 12}px;` + (ratioPad ? "position:absolute;inset:0;" : "position:static;"));
    const imgStyle = cssText(
        (m.showImagePlatform ? "filter:drop-shadow(0 6px 5px rgba(0,0,0,0.22));" : "") +
            (ratioPad ? `object-fit:${m.productPicFillType || "cover"};position:absolute;inset:0;width:100%;height:100%;display:block;` : "width:100%;height:auto;display:block;")
    );
    const priceStyle = cssText(`color:${m.productPriceColor || "#FFFFFF"};font-size:${m.priceFontSize ?? 15}px;font-weight:${Number(m.priceBold) === 0 ? 400 : 700};`);
    // 按钮三色：扁平色族跟随解析（三期，与 H5 渲染同口径）
    const globalBuyStyle = usePcGlobalBuyStyle();
    const flatBtn = resolveFlatBtnColors(m, globalBuyStyle, { gradientStart: "#FF6FA8", gradientEnd: "#FF3B8B", textColor: "#FFFFFF" });
    const buyBtnStyleObj = cssText(`background-image:linear-gradient(to right, ${flatBtn.gradientStart}, ${flatBtn.gradientEnd});color:${flatBtn.textColor};`);

    const getSales = (item: Record<string, any>) => Number(item.salesVolume ?? item.virtualSales ?? item.salesCount ?? 0);
    const getBrief = (item: Record<string, any>) => item.brief || item.sellingPoint || item.productBrief || "";
    const getFeature = (item: Record<string, any>) => item.feature || item.productFeature || item.intro || "";

    return (
        <div className="psl-root" style={outerStyle}>
            <div className="ps-wrap" style={wrapStyle}>
                {list.length === 0 ? (
                    <div className="ps-product-empty">
                        <span>请先选择商品</span>
                    </div>
                ) : (
                    <div className="ps-scroll">
                        <div className="ps-track" style={cssText(`gap:${m.productItemGap ?? 10}px;`)}>
                            {list.map((item, idx) => {
                                const a = m.cardGradientStartList?.[idx] || GRADIENT_FALLBACK[idx % GRADIENT_FALLBACK.length][0];
                                const b = m.cardGradientEndList?.[idx] || GRADIENT_FALLBACK[idx % GRADIENT_FALLBACK.length][1];
                                const percent = m.productPercent ?? 42;
                                const itemStyle = cssText(
                                    [
                                        `flex:0 0 ${percent}%;width:${percent}%;`,
                                        `background-image:linear-gradient(to bottom, ${a}, ${b});`,
                                        `border-radius:${m.cardRadius ?? 16}px;`,
                                        `gap:${m.layerGap ?? 10}px;`,
                                        `align-items:${itemsAlign};`,
                                        `text-align:${align};`
                                    ].join("")
                                );
                                const hero = m.cardTitleList?.[idx] || "";
                                const buyType = Number(m.buyBtnStyle) || 0;
                                return (
                                    <div className="ps-item" key={`${item.productId || 0}_${idx}`} style={itemStyle}>
                                        {hero ? (
                                            <div className="ps-hero" style={cssText(`color:${m.cardTitleColor || "#FFFFFF"};font-weight:${m.textWeight || 700};`)}>
                                                {hero}
                                            </div>
                                        ) : null}
                                        {Number(m.showProductName) === 1 ? (
                                            <div className="ps-name" style={cssText(`color:${m.descColor || "#FFFFFF"};`)}>
                                                {item.productName}
                                            </div>
                                        ) : null}
                                        {Number(m.showFeature) === 1 && getFeature(item) ? (
                                            <div className="ps-feature" style={cssText(`color:${m.descColor || "#FFFFFF"};`)}>
                                                {getFeature(item)}
                                            </div>
                                        ) : null}
                                        <div className="ps-photo" style={cssText(ratioPad ? `padding-top:${ratioPad};` : "padding-top:0;")}>
                                            <div className="ps-img-wrap" style={imgWrapStyle}>
                                                <img src={productImage(item)} alt="" style={imgStyle} />
                                            </div>
                                            {Number(m.showBrief) === 1 && getBrief(item) ? (
                                                <div className="ps-badge" style={cssText(`background-color:${m.badgeColor || "#FE3A01"};color:#ffffff;`)}>
                                                    {getBrief(item)}
                                                </div>
                                            ) : null}
                                        </div>
                                        <div className="ps-bottom" style={cssText(`justify-content:${itemsAlign};`)}>
                                            <div className="ps-prices">
                                                {Number(m.showPrice) === 1 ? (
                                                    <span className="ps-price-current" style={priceStyle}>
                                                        <i>¥</i>
                                                        <b>{priceFormat(Number(item.productPrice))}</b>
                                                    </span>
                                                ) : null}
                                                {Number(m.showMarketPrice) === 1 && Number(item.marketPrice) > Number(item.productPrice) ? (
                                                    <span className="ps-price-market" style={cssText(`color:${m.productMarketPriceColor};`)}>
                                                        ¥{priceFormat(Number(item.marketPrice))}
                                                    </span>
                                                ) : null}
                                                {buyType > 0 ? (
                                                    <a className={`ps-buy-btn ps-buy-btn-style__${buyType}`} style={buyBtnStyleObj}>
                                                        {buyType === 5 || buyType === 6 ? (
                                                            <span>购买</span>
                                                        ) : buyType === 7 || buyType === 8 ? (
                                                            <span>马上抢</span>
                                                        ) : BUY_ICON_PATHS[buyType] ? (
                                                            <svg viewBox="0 0 1024 1024" width="12" height="12" fill="currentColor" aria-hidden="true">
                                                                <path d={BUY_ICON_PATHS[buyType]} />
                                                            </svg>
                                                        ) : null}
                                                    </a>
                                                ) : null}
                                            </div>
                                            {Number(m.showSales) === 1 ? (
                                                <div className="ps-sales" style={cssText(`color:${m.productSalesColor};`)}>
                                                    已售{getSales(item)}
                                                </div>
                                            ) : null}
                                        </div>
                                    </div>
                                );
                            })}
                        </div>
                    </div>
                )}
            </div>
        </div>
    );
}
