/**
 * 限时特惠 预览（类名对齐 uniapp-react limitedSpecial：ls-wrap/ls-bg/ls-scroll/ls-item/ls-prices）
 */
import { useMemo } from "react";
import { cssText, imageFormat, productImage, priceFormat, pickPic } from "../../../shared/format";
import { mergeDefaultModule, formatFrame } from "../../../contract/sharedDefaults";
import { useDecorateProductList } from "../../../shared/useDecorateProductList";
import { limitedSpecialDefaults } from "./index";
import "./Preview.css";

const DEMO_PRODUCTS = [
    { productId: 9001, productName: "演示商品一", productPrice: 99, marketPrice: 129, picUrl: "https://oss.tigshop.com/img/gallery/202411/1730957947YV2lncqLSOSg8OvpbK.png", picThumb: "https://oss.tigshop.com/img/gallery/202411/1730957947YV2lncqLSOSg8OvpbK.png" },
    { productId: 9002, productName: "演示商品二", productPrice: 59, marketPrice: 79, picUrl: "https://oss.tigshop.com/img/gallery/202411/1730957948LcXRbvCuwlCHyHy7Y0.png", picThumb: "https://oss.tigshop.com/img/gallery/202411/1730957948LcXRbvCuwlCHyHy7Y0.png" },
    { productId: 9003, productName: "演示商品三", productPrice: 39, marketPrice: 49, picUrl: "https://oss.tigshop.com/img/gallery/202411/1730957949mTiXqnQpGXzYHnNV.png", picThumb: "https://oss.tigshop.com/img/gallery/202411/1730957949mTiXqnQpGXzYHnNV.png" },
];

export default function LimitedSpecialPreview({ data }: { data: Record<string, any> }) {
    const module = useMemo(() => {
        const merged = JSON.parse(JSON.stringify(data ?? {}));
        mergeDefaultModule(merged, limitedSpecialDefaults);
        return merged;
    }, [data]);
    const frameFormat = formatFrame(module.frame ?? {});
    const fetched = useDecorateProductList(module.products);
    const productList = fetched.length ? fetched : DEMO_PRODUCTS;
    const bg = pickPic(module.containerBgImage);
    return (
        <div className="module-ad-con module-limited_special" style={cssText(frameFormat.boxPadding, frameFormat.boxPaddingTop, frameFormat.boxPaddingBottom)}>
            <div className="ls-wrap" style={{ background: `linear-gradient(to right, ${module.boxGradientStart}, ${module.boxGradientEnd})`, borderRadius: `${module.frame?.boxRadius ?? 8}px` }}>
                {bg ? <img className="ls-bg" src={imageFormat(bg)} alt="" /> : null}
                <div className="ls-scroll" style={{ paddingLeft: `${module.productAreaLeft ?? 38}%`, paddingTop: `${module.productAreaPaddingY ?? 8}px`, paddingBottom: `${module.productAreaPaddingY ?? 8}px` }}>
                    <div className="ls-track" style={{ gap: `${module.productItemGap ?? 8}px` }}>
                        {productList.map((item: any, i: number) => (
                            <div className="ls-item" key={i} style={{ flex: `0 0 ${module.productPercent ?? 30}%`, backgroundColor: module.cardBgColor, borderRadius: `${module.cardRadius ?? 8}px`, padding: `${module.productItemPadding ?? 6}px` }}>
                                <div className="ls-photo"><div className="ls-img-wrap"><img className="ls-img ls-img-ratio" src={productImage(item)} alt="" style={{ borderRadius: `${module.productImgRadius ?? 8}px`, objectFit: module.productPicFillType ?? "cover" }} /></div></div>
                                <div className="ls-info" style={{ textAlign: module.textAlign ?? "center" }}>
                                    {Number(module.showProductName) === 1 ? <div className="ls-name" style={{ color: module.productTitleColor }}>{item.productName}</div> : null}
                                    {Number(module.showBrief) === 1 && module.productBriefList?.[i] ? <div className="ls-brief" style={{ background: `linear-gradient(to right, ${module.briefGradientStart}, ${module.briefGradientEnd})`, color: module.briefTextColor }}>{module.productBriefList[i]}</div> : null}
                                    <div className="ls-prices">
                                        {Number(module.showPrice) === 1 ? <span className="price-current" style={{ color: module.productPriceColor, fontSize: `${module.priceFontSize ?? 13}px`, fontWeight: Number(module.priceBold) === 1 ? 700 : 400 }}>¥{priceFormat(item.productPrice)}</span> : null}
                                        {Number(module.showMarketPrice) === 1 && item.marketPrice ? <span className="price-market" style={{ color: module.productMarketPriceColor }}>¥{priceFormat(item.marketPrice)}</span> : null}
                                    </div>
                                    {Number(module.showSales) === 1 && item.sales ? <div className="ls-extra" style={{ color: module.productSalesColor }}>已售{item.sales}</div> : null}
                                    {Number(module.buyBtnStyle) !== 0 ? <div className="ls-bottom"><span className="buy-btn-text" style={{ background: `linear-gradient(to right, ${module.buyBtnGradientStart}, ${module.buyBtnGradientEnd})`, color: module.buyBtnTextColor }}>抢购</span></div> : null}
                                </div>
                            </div>
                        ))}
                    </div>
                </div>
                <div className="ls-more">更多 &gt;</div>
            </div>
        </div>
    );
}
