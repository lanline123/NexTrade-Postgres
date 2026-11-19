/**
 * 限时特惠 预览（移植自 view/admin editor/modules/mobilePro/limitedSpecial/Preview.vue）
 *
 * 左侧「查看更多」热区（宽 productAreaLeft%）+ 右侧横滑商品轨；
 * 商品按 products 来源拉取（useDecorateProductList），无来源/无结果时用原版演示商品兜底
 * （与原版 _getProductList 的 demoProducts 回退一致）。
 * 购买钮 1~4 为 iconfont 字形，本实现以语义对应的内联 SVG 承接（同 basic/product 口径）。
 */
import { useMemo } from "react";
import { cssText, productImage, priceFormat } from "../../../shared/format";
import { useDecorateProductList } from "../../../shared/useDecorateProductList";
import { mergeDefaultModule } from "../../../contract/sharedDefaults";
import { limitedSpecialDefaults } from "./index";
import "./Preview.css";

/** 原版 Preview.vue 内置演示商品（契约冻结） */
const demoProducts: Array<Record<string, any>> = [
    {
        productId: 0,
        productName: "婴儿连体衣 会员特价",
        productPrice: 699.0,
        marketPrice: 999.0,
        salesVolume: 1200,
        commentCount: 320,
        picThumb: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/17319190474y0XWFxO8GJZF0dg64.jpg"
    },
    {
        productId: 0,
        productName: "纯棉新生儿套装 超级会员日",
        productPrice: 699.0,
        marketPrice: 999.0,
        salesVolume: 860,
        commentCount: 210,
        picThumb: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1731919071axvkfUDkQNFbIzF1iY.jpg"
    },
    {
        productId: 0,
        productName: "可折叠婴儿浴盆 限时特惠",
        productPrice: 699.0,
        marketPrice: 999.0,
        salesVolume: 540,
        commentCount: 130,
        picThumb: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1731919071lOa0ZBx2mYlvq1lgjr.jpg"
    },
    {
        productId: 0,
        productName: "婴儿洗护礼盒 热卖中",
        productPrice: 699.0,
        marketPrice: 999.0,
        salesVolume: 320,
        commentCount: 88,
        picThumb: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1731919071xUnObq2LokKPFecwXq.jpg"
    }
];

const ratioPaddingMap: Record<string, string> = {
    default: "",
    "1:1": "100%",
    "3:4": "133.333%",
    "3:2": "66.667%"
};

const CART_PATH = "M401.3 829.9c-35.8 0-64.9 29.1-64.9 64.9s29.1 64.9 64.9 64.9 64.9-29.1 64.9-64.9-29.1-64.9-64.9-64.9z m400.4 0c-35.8 0-64.9 29.1-64.9 64.9s29.1 64.9 64.9 64.9 64.9-29.1 64.9-64.9-29.1-64.9-64.9-64.9z m211.9-622.1c-11-12.7-27.2-19.9-44.1-19.9H238.9l-17.4-96.8C217.4 55.4 185.5 29 149.5 29H63.4C37.8 29 17 49.8 17 75.4s20.8 46.4 46.4 46.4h86.1c3.2 0 5.9 2.3 6.5 5.4l76.7 425.7c8.5 47.6 49.8 82.1 98.2 82.1h438.3c46.4 0 86.9-33 96.2-78.5l54.7-270.4c3.4-18.1-1.3-36.6-13.5-50.3z";
const CART2_PATH = "M896 149.3H725.3l-27.7-55.5c-8.1-16.2-24.6-26.5-42.7-26.5H369.1c-18.1 0-34.6 10.2-42.7 26.5l-27.7 55.5H128c-35.3 0-64 28.7-64 64v618.7c0 35.3 28.7 64 64 64h768c35.3 0 64-28.7 64-64V213.3c0-35.3-28.7-64-64-64z m-64 576H192V320h640v405.3z";
const PLUS_PATH = "M448 128h128v320h320v128H576v320H448V576H128V448h320V128z";
const PLUS2_PATH = "M512 64C264.6 64 64 264.6 64 512s200.6 448 448 448 448-200.6 448-448S759.4 64 512 64z m213.3 480H544v181.3c0 17.6-14.4 32-32 32s-32-14.4-32-32V544H298.7c-17.6 0-32-14.4-32-32s14.4-32 32-32H480V298.7c0-17.6 14.4-32 32-32s32 14.4 32 32V480h181.3c17.6 0 32 14.4 32 32s-14.4 32-32 32z";

function BuyBtnIcon({ style }: { style: number }) {
    const path = style === 1 ? CART_PATH : style === 2 ? CART2_PATH : style === 3 ? PLUS_PATH : PLUS2_PATH;
    return (
        <span className="ls-buy-icon">
            <svg viewBox="0 0 1024 1024" width="1em" height="1em" fill="currentColor" style={{ display: "block" }}>
                <path d={path} />
            </svg>
        </span>
    );
}

export default function LimitedSpecialPreview({ data }: { data: Record<string, any> }) {
    const module = useMemo(() => {
        const merged = JSON.parse(JSON.stringify(data ?? {}));
        mergeDefaultModule(merged, limitedSpecialDefaults);
        return merged;
    }, [data]);

    const fetched = useDecorateProductList(module.products);
    const productList = fetched.length > 0 ? fetched : demoProducts;

    const f = module.frame ?? {};
    const outerStyle = cssText(
        `padding-left:${f.boxPadding ?? 0}px;`,
        `padding-right:${f.boxPadding ?? 0}px;`,
        `padding-top:${f.boxPaddingTop ?? 0}px;`,
        `padding-bottom:${f.boxPaddingBottom ?? 0}px;`,
        f.backgroundColor ? `background-color:${f.backgroundColor};` : ""
    );

    const a = module.boxGradientStart || "#FF9EC4";
    const b = module.boxGradientEnd || "#FF6FA8";
    const r = f.boxRadius ?? 8;
    const bgImg = module.containerBgImage?.picUrl;
    const layers: string[] = [];
    const sizes: string[] = [];
    if (bgImg) {
        layers.push(`url(${bgImg})`);
        sizes.push("cover");
    }
    layers.push(`linear-gradient(to right, ${a}, ${b})`);
    sizes.push("auto");
    const py = module.productAreaPaddingY ?? 8;
    const containerBgStyle = cssText(
        `padding:${py}px 0;`,
        `background-image:${layers.join(",")};`,
        `background-size:${sizes.join(",")};`,
        "background-position:center;",
        "background-repeat:no-repeat;",
        `border-radius:${r}px;`
    );

    const left = module.productAreaLeft ?? 38;
    const trackStyle = cssText(`gap:${module.productItemGap ?? 8}px;`, `padding-left:${left}%;`);
    const moreStyle = cssText(`width:${left}%;`);
    const itemStyle = cssText(
        `background-color:${module.cardBgColor || "#FFFFFF"};`,
        `border-radius:${module.cardRadius ?? 8}px;`,
        `padding:${module.productItemPadding ?? 6}px;box-sizing:border-box;`,
        `flex:0 0 ${module.productPercent ?? 30}%;width:${module.productPercent ?? 30}%;`
    );

    const ratio = module.productPicRatio || "1:1";
    const ratioPadding = ratioPaddingMap[ratio] ?? "";
    const photoStyle = cssText(ratioPadding ? `padding-top:${ratioPadding};` : "padding-top:0;");
    const imgWrapStyle = cssText(
        `border-radius:${module.productImgRadius ?? 8}px;`,
        ratioPadding ? "position:absolute;inset:0;" : "position:static;"
    );
    const imgStyle = cssText(
        `object-fit:${module.productPicFillType || "cover"};`,
        ratioPadding ? "position:absolute;inset:0;width:100%;height:100%;display:block;" : "width:100%;height:auto;display:block;"
    );
    const briefStyle = cssText(
        `background-image:linear-gradient(to bottom, ${module.briefGradientStart || "#FF3B8B"}, ${module.briefGradientEnd || "#FF7AB0"});`,
        `color:${module.briefTextColor || "#FFFFFF"};`
    );
    const infoStyle = cssText(`text-align:${module.textAlign || "center"};`);
    const nameStyle = cssText(`color:${module.productTitleColor || "#333333"};`, `font-weight:${module.textWeight || 400};`);
    const priceStyle = cssText(
        `color:${module.productPriceColor || "#FF3B8B"};`,
        `font-size:${module.priceFontSize ?? 13}px;`,
        `font-weight:${Number(module.priceBold) === 0 ? 400 : 700};`
    );
    const buyBtnStyleObj = cssText(
        `background-image:linear-gradient(to right, ${module.buyBtnGradientStart || "#FF3B8B"}, ${module.buyBtnGradientEnd || "#FF7AB0"});`,
        `color:${module.buyBtnTextColor || "#FFFFFF"};`
    );

    const getSales = (item: any) => Number(item.salesVolume ?? item.virtualSales ?? item.salesCount ?? 0);
    const getComment = (item: any) => Number(item.commentCount ?? item.commentNumber ?? 0);
    const buyBtn = Number(module.buyBtnStyle ?? 0);

    return (
        <div className="module-ad-con module-limited_special" style={outerStyle}>
            <div className="ls-wrap" style={containerBgStyle}>
                <a className="ls-more" style={moreStyle} />
                {!productList || productList.length === 0 ? (
                    <div className="ls-product-empty">
                        <span>请先选择商品</span>
                    </div>
                ) : (
                    <div className="ls-scroll">
                        <div className="ls-track" style={trackStyle}>
                            {productList.map((item: any, idx: number) => (
                                <div className="ls-item" key={`${item.productId || 0}_${idx}`} style={itemStyle}>
                                    <div className="ls-photo" style={photoStyle}>
                                        <div className="ls-img-wrap" style={imgWrapStyle}>
                                            <img src={productImage(item)} alt="" style={imgStyle} />
                                        </div>
                                        {Number(module.showBrief) === 1 && module.productBriefList?.[idx] ? (
                                            <div className="ls-brief" style={briefStyle}>
                                                {module.productBriefList[idx]}
                                            </div>
                                        ) : null}
                                    </div>
                                    <div className="ls-info" style={infoStyle}>
                                        {Number(module.showProductName) === 1 ? (
                                            <div className="ls-name" style={nameStyle}>
                                                {item.productName}
                                            </div>
                                        ) : null}
                                        <div className="ls-bottom">
                                            <div className="ls-prices">
                                                {Number(module.showPrice) === 1 ? (
                                                    <span className="ls-price-current" style={priceStyle}>
                                                        <i>¥</i>
                                                        <b>{priceFormat(Number(item.productPrice))}</b>
                                                    </span>
                                                ) : null}
                                                {Number(module.showMarketPrice) === 1 && Number(item.marketPrice) > Number(item.productPrice) ? (
                                                    <span className="ls-price-market" style={{ color: module.productMarketPriceColor }}>
                                                        ¥{priceFormat(Number(item.marketPrice))}
                                                    </span>
                                                ) : null}
                                                {buyBtn > 0 ? (
                                                    <a className={`ls-buy-btn ls-buy-btn-style__${buyBtn}`} style={buyBtnStyleObj}>
                                                        {buyBtn === 5 || buyBtn === 6 ? (
                                                            <span>购买</span>
                                                        ) : buyBtn === 7 || buyBtn === 8 ? (
                                                            <span>马上抢</span>
                                                        ) : (
                                                            <BuyBtnIcon style={buyBtn} />
                                                        )}
                                                    </a>
                                                ) : null}
                                            </div>
                                            {Number(module.showSales) === 1 || Number(module.showComment) === 1 ? (
                                                <div className="ls-extra" style={{ color: module.productSalesColor }}>
                                                    {Number(module.showSales) === 1 ? <span>已售{getSales(item)}</span> : null}
                                                    {Number(module.showComment) === 1 ? <span>{getComment(item)}评价</span> : null}
                                                </div>
                                            ) : null}
                                        </div>
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
